use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("BRssGR7YU8i2AJ8um42m31qJR2mZW8MMqcKCQ35uH8x5");

#[program]
pub mod token_quest {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.bump = ctx.bumps.state;
        state.fee_percentage = 300;
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
        let staking_period: i64 = 60; // seconds
        let locking_period: i64 = 30; // seconds
        require!(
            current_time >= stake_pda.stake_timestamp + staking_period + locking_period,
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
        let _gvault_seeds: &[&[u8]] = &[b"vault", b"sol", &[ctx.bumps.vault_pda]];
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

    pub fn deposit_spl(ctx: Context<DepositSPL>, amount: u64) -> Result<()> {
        // Step 1️⃣ — Basic validation
        require!(amount > 0, CustomError::InvalidAmount);

        // Step 2️⃣ — Prepare all accounts we’ll use
        let user = &ctx.accounts.user;
        let user_token_account = &ctx.accounts.user_token_account;
        let vault_pda = &ctx.accounts.vault_pda;
        let stake_pda = &mut ctx.accounts.stake_pda;
        let token_program = &ctx.accounts.token_program;

        // Step 3️⃣ — Transfer tokens from user to vault PDA
        // This uses Anchor’s built-in CPI to the SPL Token Program.
        let cpi_accounts = anchor_spl::token::Transfer {
            from: user_token_account.to_account_info(),
            to: vault_pda.to_account_info(),
            authority: user.to_account_info(),
        };

        let cpi_context = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        anchor_spl::token::transfer(cpi_context, amount)?;

        // Step 4️⃣ — Record deposit in the stake_pda
        stake_pda.user = user.key();
        stake_pda.amount = amount;
        stake_pda.stake_timestamp = Clock::get()?.unix_timestamp;
        stake_pda.is_withdrawn = false;
        stake_pda.bump = *ctx.bumps.get("stake_pda").unwrap();

        msg!("✅ {} tokens deposited by {}", amount, user.key());

        Ok(())
    }

    pub fn withdraw_spl(ctx: Context<WithdrawSPL>) -> Result<()> {
        let user = &ctx.accounts.user;
        let state = &ctx.accounts.state;
        let stake_pda = &mut ctx.accounts.stake_pda;
        let vault_pda = &ctx.accounts.vault_pda;
        let user_token_account = &ctx.accounts.user_token_account;
        let token_program = &ctx.accounts.token_program;
        let clock = Clock::get()?;

        // 1️⃣ Check if already withdrawn
        require!(!stake_pda.is_withdrawn, CustomError::AlreadyWithdrawn);

        // 2️⃣ Check time — ensure staking + locking period has passed
        let unlock_time = stake_pda.stake_timestamp + STAKING_PERIOD + LOCKING_PERIOD;
        require!(
            clock.unix_timestamp >= unlock_time,
            CustomError::LockPeriodNotOver
        );

        // 3️⃣ Calculate amount and optional fee
        let mut withdraw_amount = stake_pda.amount;
        let mut fee_amount = 0u64;

        if state.fee_percentage > 0 {
            fee_amount = (withdraw_amount * state.fee_percentage as u64) / 10_000;
            withdraw_amount = withdraw_amount - fee_amount;
        }

        // 4️⃣ Transfer tokens from vault PDA -> user’s token account
        let seeds = &[
            b"vault",
            ctx.accounts.mint.key().as_ref(),
            &[ctx.bumps.vault_pda],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_to_user = anchor_spl::token::Transfer {
            from: vault_pda.to_account_info(),
            to: user_token_account.to_account_info(),
            authority: vault_pda.to_account_info(),
        };

        let cpi_ctx_user = CpiContext::new_with_signer(
            token_program.to_account_info(),
            transfer_to_user,
            signer_seeds,
        );

        anchor_spl::token::transfer(cpi_ctx_user, withdraw_amount)?;

        // 5️⃣ Optional fee transfer (vault -> fee PDA)
        if fee_amount > 0 {
            let transfer_to_fee = anchor_spl::token::Transfer {
                from: vault_pda.to_account_info(),
                to: ctx.accounts.fee_pda.to_account_info(),
                authority: vault_pda.to_account_info(),
            };

            let cpi_ctx_fee = CpiContext::new_with_signer(
                token_program.to_account_info(),
                transfer_to_fee,
                signer_seeds,
            );

            anchor_spl::token::transfer(cpi_ctx_fee, fee_amount)?;
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
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 1 + 2,
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
        seeds = [b"stake_pda", user.key().as_ref()],
        bump,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct StakeAccount {
    user: Pubkey,
    amount: u64,
    stake_timestamp: i64,
    is_withdrawn: bool,
    bump: u8,
}

#[derive(Accounts)]
pub struct WithdrawSOL<'info> {
    /// The user requesting withdrawal (must sign)
    #[account(mut)]
    pub user: Signer<'info>,

    /// The user's stake record PDA
    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref()],
        bump,
        has_one = user,
        // if you want to auto-refund rent: use `close = user`
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    /// Program SOL vault PDA (holds lamports). Use UncheckedAccount if it's only lamports.
    #[account(
        mut,
        seeds = [b"vault", b"sol"],
        bump,
    )]
    /// CHECK: This is a PDA owned by the program and used to hold lamports.
    pub vault_pda: UncheckedAccount<'info>,

    /// Fee PDA for SOL (optional)
    #[account(
        mut,
        seeds = [b"fee", b"sol"],
        bump,
    )]
    /// CHECK: PDA used to receive fees
    pub fee_pda: UncheckedAccount<'info>,

    /// Program config
    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    /// Clock sysvar (read-only)
    pub clock: Sysvar<'info, Clock>,

    /// System program for lamport transfers
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSPL<'info> {
    // 1️⃣ The user who deposits SPL tokens
    #[account(mut)]
    pub user: Signer<'info>,

    // 2️⃣ The user's SPL token account — the source of tokens
    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == mint.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    // 3️⃣ The mint that defines which SPL token (like USDC, MYCOIN)
    pub mint: Account<'info, Mint>,

    // 4️⃣ Program-owned PDA token account to store deposited tokens
    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump,
        constraint = vault_pda.mint == mint.key(),
    )]
    pub vault_pda: Account<'info, TokenAccount>,

    // 5️⃣ Record for user’s deposit — stores stake info
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 1 + 1,
        seeds = [b"stake", user.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    // 6️⃣ Program-wide config (for fees, etc.)
    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    // 7️⃣ Required programs and system variables
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct WithdrawSPL<'info> {
    /// 🧍 The user requesting withdrawal
    #[account(mut)]
    pub user: Signer<'info>,

    /// 💾 The stake PDA created during deposit
    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref(), mint.key().as_ref()],
        bump,
        has_one = user,
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    /// 🪙 The SPL mint of the token being withdrawn (like USDC or MYCOIN)
    pub mint: Account<'info, Mint>,

    /// 🏦 Program-owned token vault PDA holding deposited tokens
    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump,
        constraint = vault_pda.mint == mint.key(),
    )]
    pub vault_pda: Account<'info, TokenAccount>,

    /// 💰 User’s token account to receive withdrawn tokens
    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == mint.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// 💸 Optional fee receiver PDA (if fee-on-withdrawal enabled)
    #[account(
        mut,
        seeds = [b"fee", mint.key().as_ref()],
        bump,
    )]
    /// CHECK: Only holds tokens, not arbitrary data
    pub fee_pda: UncheckedAccount<'info>,

    /// ⚙️ Global program configuration (admin, fees, etc.)
    #[account(mut)]
    pub state: Account<'info, TokenQuestState>,

    /// ⏰ For time-based checks (staking duration)
    pub clock: Sysvar<'info, Clock>,

    /// 🔁 SPL Token program
    pub token_program: Program<'info, Token>,

    /// 🧱 System program (for rent refunds if needed)
    pub system_program: Program<'info, System>,
}
