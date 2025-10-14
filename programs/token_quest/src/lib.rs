use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

declare_id!("BFMcCzgiUKB3mZ2eT1AwgTdHeE3ozhe1cp67uzvbEwm6");

// Define constants for staking and locking periods
const STAKING_PERIOD: i64 = 60; // seconds
const LOCKING_PERIOD: i64 = 30; // seconds

#[program]
pub mod token_quest {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.bump = ctx.bumps.state;
        state.fee_percentage = 300;
        state.user_tax_on_withdraw = false; // Initialize this field
        msg!(
            "TokenQuest initialized by admin: {}",
            ctx.accounts.admin.key()
        );
        Ok(())
    }

    pub fn deposit_sol(ctx: Context<DepositSOL>, amount: u64) -> Result<()> {
        let user = &ctx.accounts.user;
        let vault_pda = &ctx.accounts.vault_pda;
        let stake_pda = &mut ctx.accounts.stake_pda;
        let system_program = &ctx.accounts.system_program;

        // 1️⃣ Check that the amount is valid
        require!(amount > 0, CustomError::InvalidAmount);

        // 2️⃣ Transfer SOL from user to vault PDA
        let transfer_ix = anchor_lang::system_program::Transfer {
            from: user.to_account_info(),
            to: vault_pda.to_account_info(),
        };

        anchor_lang::system_program::transfer(
            CpiContext::new(system_program.to_account_info(), transfer_ix),
            amount,
        )?;

        // 3️⃣ Record deposit details in the stake_pda
        stake_pda.user = user.key();
        stake_pda.amount = amount;
        stake_pda.stake_timestamp = Clock::get()?.unix_timestamp;
        stake_pda.is_withdrawn = false;
        stake_pda.bump = ctx.bumps.stake_pda;

        msg!("{} lamports deposited by {}", amount, user.key());

        Ok(())
    }

    pub fn withdraw_sol(ctx: Context<WithdrawSOL>) -> Result<()> {
        let user = &ctx.accounts.user;
        let vault_pda = &ctx.accounts.vault_pda;
        let stake_pda = &mut ctx.accounts.stake_pda;
        let state = &ctx.accounts.state;
        let clock = &ctx.accounts.clock;

        // 1️⃣ Validate ownership — only stake owner can withdraw
        require!(stake_pda.user == user.key(), CustomError::UnauthorizedUser);

        // 2️⃣ Prevent double withdrawals
        require!(!stake_pda.is_withdrawn, CustomError::AlreadyWithdrawn);

        // 3️⃣ Check time lock (staking period + locking period)
        let current_time = clock.unix_timestamp;
        require!(
            current_time >= stake_pda.stake_timestamp + STAKING_PERIOD + LOCKING_PERIOD,
            CustomError::LockNotEnded
        );

        // 4️⃣ Check vault has enough SOL
        let amount = stake_pda.amount;
        require!(
            vault_pda.lamports() >= amount,
            CustomError::InsufficientVaultBalance
        );

        // 5️⃣ Calculate withdraw fee (if enabled)
        let mut fee: u64 = 0;
        if state.user_tax_on_withdraw {
            fee = amount.checked_mul(state.fee_percentage as u64).unwrap() / 10_000;
        }
        let payout = amount.checked_sub(fee).unwrap();

        // 6️⃣ Transfer SOL from vault PDA → user (signed by PDA)
        **vault_pda.to_account_info().try_borrow_mut_lamports()? -= payout;
        **user.to_account_info().try_borrow_mut_lamports()? += payout;

        // 7️⃣ Transfer fee (if any) to fee_pda
        if fee > 0 {
            let fee_pda = &ctx.accounts.fee_pda;
            **vault_pda.to_account_info().try_borrow_mut_lamports()? -= fee;
            **fee_pda.to_account_info().try_borrow_mut_lamports()? += fee;
        }

        // 8️⃣ Mark stake as withdrawn
        stake_pda.is_withdrawn = true;

        msg!(
            "✅ {} lamports withdrawn by {} (fee: {})",
            payout,
            user.key(),
            fee
        );

        Ok(())
    }

    pub fn withdraw_fees_sol(ctx: Context<WithdrawFeeSOL>) -> Result<()> {
        let fee_pda = &ctx.accounts.fee_pda;
        let admin = &ctx.accounts.admin;

        // Get the fee PDA balance
        let fee_balance = fee_pda.lamports();

        require!(fee_balance > 0, CustomError::NoFeesToWithdraw);

        // Transfer all fees from fee_pda to admin
        **fee_pda.to_account_info().try_borrow_mut_lamports()? -= fee_balance;
        **admin.to_account_info().try_borrow_mut_lamports()? += fee_balance;

        msg!("✅ Admin withdrew {} lamports in fees", fee_balance);

        Ok(())
    }

    pub fn deposit_spl(ctx: Context<DepositSPL>, amount: u64) -> Result<()> {
        // Step 1️⃣ — Basic validation
        require!(amount > 0, CustomError::InvalidAmount);

        // Step 2️⃣ — Prepare all accounts we'll use
        let user = &ctx.accounts.user;
        let stake_pda = &mut ctx.accounts.stake_pda;

        // Step 3️⃣ — Transfer tokens from user to vault PDA using anchor_spl
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_pda.to_account_info(),
            authority: user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        anchor_spl::token::transfer(cpi_ctx, amount)?;

        // Step 4️⃣ — Record deposit in the stake_pda
        stake_pda.user = user.key();
        stake_pda.amount = amount;
        stake_pda.stake_timestamp = Clock::get()?.unix_timestamp;
        stake_pda.is_withdrawn = false;
        stake_pda.bump = ctx.bumps.stake_pda;

        msg!("✅ {} tokens deposited by {}", amount, user.key());

        Ok(())
    }

    pub fn withdraw_spl(ctx: Context<WithdrawSPL>) -> Result<()> {
        let user = &ctx.accounts.user;
        let state = &ctx.accounts.state;
        let stake_pda = &mut ctx.accounts.stake_pda;
        let clock = Clock::get()?;

        // 1️⃣ Check if already withdrawn
        require!(!stake_pda.is_withdrawn, CustomError::AlreadyWithdrawn);

        // 2️⃣ Check time — ensure staking + locking period has passed
        let unlock_time = stake_pda.stake_timestamp + STAKING_PERIOD + LOCKING_PERIOD;
        require!(
            clock.unix_timestamp >= unlock_time,
            CustomError::LockNotEnded
        );

        // 3️⃣ Calculate amount and optional fee
        let mut withdraw_amount = stake_pda.amount;
        let mut fee_amount = 0u64;

        if state.fee_percentage > 0 && state.user_tax_on_withdraw {
            fee_amount = (withdraw_amount * state.fee_percentage as u64) / 10_000;
            withdraw_amount = withdraw_amount.checked_sub(fee_amount).unwrap();
        }

        // 4️⃣ Transfer tokens from vault PDA -> user's token account
        let mint_key = ctx.accounts.mint.key();
        let seeds = &[b"vault", mint_key.as_ref(), &[ctx.bumps.vault_pda]];
        let signer_seeds = &[&seeds[..]];

        let transfer_to_user = token::Transfer {
            from: ctx.accounts.vault_pda.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_pda.to_account_info(),
        };

        let cpi_ctx_user = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_to_user,
            signer_seeds,
        );

        token::transfer(cpi_ctx_user, withdraw_amount)?;

        // 5️⃣ Optional fee transfer (vault -> fee PDA)
        if fee_amount > 0 {
            let transfer_to_fee = token::Transfer {
                from: ctx.accounts.vault_pda.to_account_info(),
                to: ctx.accounts.fee_pda.to_account_info(),
                authority: ctx.accounts.vault_pda.to_account_info(),
            };

            let cpi_ctx_fee = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_to_fee,
                signer_seeds,
            );

            token::transfer(cpi_ctx_fee, fee_amount)?;
        }

        // 6️⃣ Mark stake as withdrawn
        stake_pda.is_withdrawn = true;

        msg!(
            "✅ User {} successfully withdrew {} tokens",
            user.key(),
            withdraw_amount
        );

        Ok(())
    }

    pub fn withdraw_fees_spl(ctx: Context<WithdrawFeeSPL>) -> Result<()> {
        let fee_pda = &ctx.accounts.fee_pda;
        let fee_balance = fee_pda.amount;

        require!(fee_balance > 0, CustomError::NoFeesToWithdraw);

        // Prepare PDA signer seeds
        let mint_key = ctx.accounts.mint.key();
        let seeds = &[b"fee", mint_key.as_ref(), &[ctx.bumps.fee_pda]];
        let signer_seeds = &[&seeds[..]];

        // Transfer all fees from fee_pda to admin's token account
        let transfer_fees = token::Transfer {
            from: fee_pda.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
            authority: fee_pda.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_fees,
            signer_seeds,
        );

        token::transfer(cpi_ctx, fee_balance)?;

        msg!("✅ Admin withdrew {} tokens in fees", fee_balance);

        Ok(())
    }
}

#[error_code]
pub enum CustomError {
    #[msg("Invalid deposit amount.")]
    InvalidAmount,
    #[msg("Unauthorized user.")]
    UnauthorizedUser,
    #[msg("Stake already withdrawn.")]
    AlreadyWithdrawn,
    #[msg("Lock period has not ended yet.")]
    LockNotEnded,
    #[msg("Vault does not have enough SOL.")]
    InsufficientVaultBalance,
    #[msg("No fees available to withdraw.")]
    NoFeesToWithdraw,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 1 + 2 + 1, // Added 1 byte for bool
        seeds = [b"state"],
        bump,
    )]
    pub state: Account<'info, TokenQuestState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct TokenQuestState {
    pub admin: Pubkey,
    pub bump: u8,
    pub fee_percentage: u16,
    pub user_tax_on_withdraw: bool,
}

#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", b"sol"],
        bump,
    )]
    pub vault_pda: SystemAccount<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 1 + 1,
        seeds = [b"stake", user.key().as_ref()], // Changed from b"stake_pda"
        bump,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct StakeAccount {
    pub user: Pubkey,
    pub amount: u64,
    pub stake_timestamp: i64,
    pub is_withdrawn: bool,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref()],
        bump,
        has_one = user,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    #[account(
        mut,
        seeds = [b"vault", b"sol"],
        bump,
    )]
    /// CHECK: This is a PDA owned by the program and used to hold lamports.
    pub vault_pda: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"fee", b"sol"],
        bump,
    )]
    /// CHECK: PDA used to receive fees
    pub fee_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    pub clock: Sysvar<'info, Clock>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawFeeSOL<'info> {
    #[account(
        mut,
        constraint = state.admin == admin.key() @ CustomError::UnauthorizedUser
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"fee", b"sol"],
        bump,
    )]
    /// CHECK: PDA used to hold fee lamports
    pub fee_pda: UncheckedAccount<'info>,

    pub state: Account<'info, TokenQuestState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSPL<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == mint.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump,
        constraint = vault_pda.mint == mint.key(),
    )]
    pub vault_pda: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 1 + 1,
        seeds = [b"stake", user.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct WithdrawSPL<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref(), mint.key().as_ref()],
        bump,
        has_one = user,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump,
        constraint = vault_pda.mint == mint.key(),
    )]
    pub vault_pda: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == mint.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"fee", mint.key().as_ref()],
        bump,
    )]
    pub fee_pda: Account<'info, TokenAccount>,

    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawFeeSPL<'info> {
    #[account(
        mut,
        constraint = state.admin == admin.key() @ CustomError::UnauthorizedUser
    )]
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"fee", mint.key().as_ref()],
        bump,
        constraint = fee_pda.mint == mint.key(),
    )]
    pub fee_pda: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = admin_token_account.owner == admin.key(),
        constraint = admin_token_account.mint == mint.key(),
    )]
    pub admin_token_account: Account<'info, TokenAccount>,

    pub state: Account<'info, TokenQuestState>,

    pub token_program: Program<'info, Token>,
}
