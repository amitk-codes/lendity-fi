use std::f64::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    error::ErrorCode, Bank, User, MAX_AGE_PYTH, SOL_USD_FEED_ID_HEX, USDC_USD_FEED_ID_HEX,
};

#[derive(Accounts)]
pub struct Borrow<'info> {
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

    pub price_update: Account<'info, PriceUpdateV2>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn borrow_handler(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    let bank = &mut ctx.accounts.bank;
    let price_update = &mut ctx.accounts.price_update;
    let total_collateral: u64;

    // calculating the borrowable amount :-
    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            let sol_usd_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID_HEX)?;
            let sol_usd_price = price_update.get_price_no_older_than(
                &Clock::get()?,
                MAX_AGE_PYTH,
                &sol_usd_feed_id,
            )?;

            let total_deposit_with_interest_accumulated =
                calculate_total_deposit_with_interest_accumulated(
                    user.last_updated,
                    user.deposited_sol,
                    bank.interest_rate,
                )?;
            total_collateral = sol_usd_price.price as u64 * total_deposit_with_interest_accumulated;
        }
        _ => {
            let usdc_usd_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID_HEX)?;
            let usdc_usd_price = price_update.get_price_no_older_than(
                &Clock::get()?,
                MAX_AGE_PYTH,
                &usdc_usd_feed_id,
            )?;

            let total_deposit_with_interest_accumulated =
                calculate_total_deposit_with_interest_accumulated(
                    user.last_updated,
                    user.deposited_usdc,
                    bank.interest_rate,
                )?;
            total_collateral =
                usdc_usd_price.price as u64 * total_deposit_with_interest_accumulated;
        }
    }

    let borrowable_amount = total_collateral * bank.liquidation_threshold;
    if borrowable_amount < amount {
        return Err(ErrorCode::OverBorrowableAmount.into());
    }

    // borrow transfer cpi :-

    let borrow_transfer_accounts = TransferChecked {
        from: ctx.accounts.bank_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.bank_token_account.to_account_info(),
    };

    let mint_key = ctx.accounts.mint.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        b"bank_token_account",
        mint_key.as_ref(),
        &[ctx.bumps.bank_token_account],
    ]];


    let borrow_cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), borrow_transfer_accounts).with_signer(signer_seeds);
    transfer_checked(borrow_cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    // states update :-

    let mut total_borrowed = bank.total_borrowed;
    let mut total_borrowed_shares = bank.total_borrowed_shares;

    if total_borrowed == 0 {
        total_borrowed = amount;
        total_borrowed_shares = amount;
    }

    let borrow_ratio = amount.checked_div(total_borrowed).unwrap();
    let user_shares = total_borrowed_shares.checked_mul(borrow_ratio).unwrap();

    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.borrowed_usdc += amount;
            user.borrowed_usdc_shares += user_shares;
        }
        _ => {
            user.borrowed_sol += amount;
            user.borrowed_sol_shares += user_shares;
        }
    }

    bank.total_borrowed += amount;
    bank.total_borrowed_shares += user_shares;

    user.last_updated_borrowed = Clock::get()?.unix_timestamp;

    Ok(())
}

pub fn calculate_total_deposit_with_interest_accumulated(
    last_updated: i64,
    total_deposit: u64,
    interest_rate: f64,
) -> Result<u64> {
    let time_diff = Clock::get()?.unix_timestamp - last_updated;
    let total_deposit_with_interest_accumulated =
        (total_deposit as f64 * E.powf(interest_rate * time_diff as f64)) as u64;
    Ok(total_deposit_with_interest_accumulated)
}
