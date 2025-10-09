use anchor_lang::prelude::*;

declare_id!("7Sowe2d6E9sZjZ4Tz35xo68KSKbo8LJi1c7MDm42vkoc");

#[program]
pub mod token_quest {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.bump = ctx.bumps.state;
        state.fee_percentage = 300; // 3%
        msg!(
            "✅ TokenQuest initialized by admin: {}",
            ctx.accounts.admin.key()
        );
        Ok(())
    }

    pub fn deposit_sol(ctx: Context<DepositSOL>, amount: u64) -> Result<()> {
        msg!("💰 Starting SOL deposit of {} lamports", amount);

        // Transfer SOL from user → vault PDA
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.vault_pda.key(),
            amount,
        );

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.vault_pda.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Record stake details
        let stake = &mut ctx.accounts.stake_pda;
        stake.user = ctx.accounts.user.key();
        stake.amount = amount;
        stake.timestamp = Clock::get()?.unix_timestamp;
        stake.is_withdrawn = false;
        stake.bump = ctx.bumps.state;

        msg!(
            "✅ Deposit complete. User: {}, Amount: {} lamports",
            stake.user,
            stake.amount
        );

        Ok(())
    }

    pub fn withdraw_sol(ctx: Context<WithdrawSOL>) -> Result<()> {
        msg!("Withdrawing solana");
        Ok(())
    }

    pub fn deposit_spl(ctx: Context<DepositSPL>) -> Result<()> {
        msg!("Depositing SPL");
        Ok(())
    }

    pub fn withdraw_spl(ctx: Context<WithdrawSPL>) -> Result<()> {
        msg!("Withdrawing SPL");
        Ok(())
    }
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
    pub admin: Signer<'info>, // ✅ rename to admin

    pub system_program: Program<'info, System>,
}

#[account]
pub struct TokenQuestState {
    pub admin: Pubkey,
    pub bump: u8,
    pub fee_percentage: u16,
}

#[derive(Accounts)]
pub struct DepositSOL<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// Vault PDA to store all SOL deposits
    #[account(
        mut,
        seeds = [b"vault", user.key().as_ref()],
        bump,
    )]
    pub vault_pda: AccountInfo<'info>,

    /// Stake PDA — records this user's individual deposit
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 1 + 1,
        seeds = [b"stake", user.key().as_ref(), Clock::get()?.unix_timestamp.to_le_bytes().as_ref()],
        bump
    )]
    pub stake_pda: Account<'info, StakeAccount>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct StakeAccount {
    pub user: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub is_withdrawn: bool,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct WithdrawSOL {}

#[derive(Accounts)]
pub struct DepositSPL {}

#[derive(Accounts)]
pub struct WithdrawSPL {}
