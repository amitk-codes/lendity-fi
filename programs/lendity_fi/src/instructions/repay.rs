use std::f64::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{error::ErrorCode, Bank, User};

#[derive(Accounts)]
pub struct Repay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [mint.key().as_ref()],
        bump,
    )]
    pub bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds = [signer.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"bank_token_account", mint.key().as_ref()],
        bump
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn repay_handler(ctx: Context<Repay>, amount: u64) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    let bank = &mut ctx.accounts.bank;

    let borrowed_value: u64;

    if ctx.accounts.mint.to_account_info().key() == user.usdc_address {
        borrowed_value = user.borrowed_usdc
    } else {
        borrowed_value = user.borrowed_sol
    }

    let time_diff = Clock::get()?.unix_timestamp - user.last_updated_borrowed;
    let total_borrowed_with_interest_accumulated =
        bank.total_borrowed as f64 * E.powf(bank.interest_rate * time_diff as f64);

    let value_per_share =
        total_borrowed_with_interest_accumulated / bank.total_borrowed_shares as f64;

    let user_accumulated_amount = borrowed_value as f64 / value_per_share;

    if amount as f64 > user_accumulated_amount {
        return Err(ErrorCode::OverRepayAmount.into());
    }

    // transfer cpi:-

    let repay_transfer_accounts = TransferChecked {
        from: ctx.accounts.user_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.bank_token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };

    let repay_transfer_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        repay_transfer_accounts,
    );

    transfer_checked(repay_transfer_cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    // states update :-

    let borrow_ratio = amount
        .checked_div(total_borrowed_with_interest_accumulated as u64)
        .unwrap();
    let user_shares = bank
        .total_borrowed_shares
        .checked_mul(borrow_ratio)
        .unwrap();

    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.borrowed_usdc -= amount;
            user.borrowed_usdc_shares -= user_shares;
        }
        _ => {
            user.borrowed_sol -= amount;
            user.borrowed_sol_shares -= user_shares;
        }
    }

    bank.total_borrowed -= amount;
    bank.total_borrowed_shares -= user_shares;

    Ok(())
}
