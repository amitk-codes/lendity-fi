use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
}};

use crate::{Bank, User};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
        mut,
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
    pub associated_token_program: Program<'info, AssociatedToken>
}

pub fn deposit_handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let deposit_transfer_accounts = TransferChecked {
        from: ctx.accounts.user_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.bank_token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };

    let deposit_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        deposit_transfer_accounts,
    );

    transfer_checked(deposit_cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    // calculating the shares :-
    let bank = &mut ctx.accounts.bank;
    let user = &mut ctx.accounts.user_account;

    let mut total_deposits = bank.total_deposits;
    let mut total_deposits_shares = bank.total_deposits_shares;

    if total_deposits == 0 {
        total_deposits = amount;
        total_deposits_shares = amount;
    }

    let deposit_ratio = amount.checked_div(total_deposits).unwrap();
    let user_shares = total_deposits_shares.checked_mul(deposit_ratio).unwrap();

    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.deposited_usdc += amount;
            user.deposited_usdc_shares += user_shares;
        }
        _ => {
            user.deposited_sol += amount;
            user.deposited_sol_shares += user_shares;
        }
    }

    bank.total_deposits += amount;
    bank.total_deposits_shares += user_shares;

    Ok(())
}
