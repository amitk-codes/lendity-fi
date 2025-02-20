pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BUmmLcAocQGUHYn96oMpGJSX6xXGtG22pZNHNNB79C94");

#[program]
pub mod lendity_fi {
    use super::*;

    pub fn initialize_bank(
        ctx: Context<InitializeBank>,
        liquidation_threshold: u64,
        max_ltv: u64,
    ) -> Result<()> {
        initialize_bank_handler(ctx, liquidation_threshold, max_ltv)?;
        Ok(())
    }

    pub fn initialize_user(ctx: Context<InitializeUser>, usdc_address: Pubkey) -> Result<()> {
        initialize_user_handler(ctx, usdc_address)?;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        deposit_handler(ctx, amount)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        withdraw_handler(ctx, amount)?;
        Ok(())
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
        borrow_handler(ctx, amount)?;
        Ok(())
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
        repay_handler(ctx, amount)?;
        Ok(())
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        liquidate_handler(ctx)?;
        Ok(())
    }
}
