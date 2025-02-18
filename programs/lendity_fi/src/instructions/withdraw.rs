use std::f64::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{error::ErrorCode, Bank, User};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

pub fn withdraw_handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    let bank = &mut ctx.accounts.bank;

    let deposited_amount: u64;
    if ctx.accounts.mint.to_account_info().key() == user.usdc_address {
        deposited_amount = user.deposited_usdc;
    } else {
        deposited_amount = user.deposited_sol;
    }

    let time_diff = user.last_updated - Clock::get()?.unix_timestamp;
    let total_deposit = bank.total_deposits as f64 * E.powf(bank.interest_rate * time_diff as f64);

    let value_per_share = total_deposit / (bank.total_deposits_shares as f64);

    let user_accumulated_amount = (deposited_amount as f64) / value_per_share;

    if user_accumulated_amount < (amount as f64) {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    if amount > deposited_amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }

    let withdraw_transfer_accounts = TransferChecked {
        from: ctx.accounts.bank_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.bank_token_account.to_account_info(),
    };

    // seeds = [b"bank_token_account", mint.key().as_ref()],
    let mint_key = ctx.accounts.mint.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        b"bank_token_account",
        mint_key.as_ref(),
        &[ctx.bumps.bank_token_account],
    ]];

    let withdraw_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        withdraw_transfer_accounts,
    )
    .with_signer(signer_seeds);

    transfer_checked(withdraw_cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    let shares_to_remove = (amount / bank.total_deposits) * bank.total_deposits_shares;

    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.deposited_usdc -= amount;
            user.deposited_usdc_shares -= shares_to_remove;
        }
        _ => {
            user.deposited_sol -= amount;
            user.deposited_sol_shares -= shares_to_remove;
        }
    }

    bank.total_deposits -= amount;
    bank.total_deposits_shares -= shares_to_remove;

    Ok(())
}
