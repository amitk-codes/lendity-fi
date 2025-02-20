use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{
    error::ErrorCode, Bank, User, MAX_AGE_PYTH, SOL_USD_FEED_ID_HEX, USDC_USD_FEED_ID_HEX,
};

use super::calculate_total_deposit_with_interest_accumulated;

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(mut)]
    pub liquidator: Signer<'info>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    pub borrowed_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds = [b"bank_token_account", collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [borrowed_mint.key().as_ref()],
        bump
    )]
    pub borrowed_bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds = [b"bank_token_account", borrowed_mint.key().as_ref()],
        bump
    )]
    pub borrowed_bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [liquidator.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(
        init_if_needed,
        payer = liquidator,
        associated_token::mint = collateral_mint,
        associated_token::authority = liquidator,
        associated_token::token_program = token_program
    )]
    pub liquidator_collateral_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = liquidator,
        associated_token::mint = borrowed_mint,
        associated_token::authority = liquidator,
        associated_token::token_program = token_program
    )]
    pub liquidator_borrowed_token_account: InterfaceAccount<'info, TokenAccount>,

    pub price_update: Account<'info, PriceUpdateV2>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn liquidate_handler(ctx: Context<Liquidate>) -> Result<()> {
    let user = &mut ctx.accounts.user_account;
    let price_update = &mut ctx.accounts.price_update;
    let collateral_bank = &mut ctx.accounts.collateral_bank;
    let borrowed_bank = &mut ctx.accounts.borrowed_bank;
    let total_borrowed: u64;
    let total_collateral: u64;

    let sol_usd_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID_HEX)?;
    let sol_usd_price_response =
        price_update.get_price_no_older_than(&Clock::get()?, MAX_AGE_PYTH, &sol_usd_feed_id)?;
    let sol_usd_price = sol_usd_price_response.price;

    let usdc_usd_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID_HEX)?;
    let usdc_usd_price_response =
        price_update.get_price_no_older_than(&Clock::get()?, MAX_AGE_PYTH, &usdc_usd_feed_id)?;
    let usdc_usd_price = usdc_usd_price_response.price;

    match ctx.accounts.collateral_mint.to_account_info().key() {
        key if key == user.usdc_address => {
            let collateral_accumulated_value = calculate_total_deposit_with_interest_accumulated(
                user.last_updated,
                user.deposited_usdc,
                collateral_bank.interest_rate,
            )?;
            total_collateral = collateral_accumulated_value * usdc_usd_price as u64;

            let borrowed_accumulated_value = calculate_total_deposit_with_interest_accumulated(
                user.last_updated_borrowed,
                user.borrowed_sol,
                borrowed_bank.interest_rate,
            )?;
            total_borrowed = borrowed_accumulated_value * sol_usd_price as u64;
        }
        _ => {
            let collateral_accumulated_value = calculate_total_deposit_with_interest_accumulated(
                user.last_updated,
                user.deposited_sol,
                collateral_bank.interest_rate,
            )?;
            total_collateral = collateral_accumulated_value * sol_usd_price as u64;

            let borrowed_accumulated_value = calculate_total_deposit_with_interest_accumulated(
                user.last_updated_borrowed,
                user.borrowed_usdc,
                borrowed_bank.interest_rate,
            )?;
            total_borrowed = borrowed_accumulated_value * usdc_usd_price as u64;
        }
    }

    // check whether the account falls below the health factor or not :-

    let health_factor = (total_collateral as f64 * collateral_bank.liquidation_threshold as f64)
        / total_borrowed as f64;

    if health_factor >= 1.0 {
        return Err(ErrorCode::DoesNotFallBelowHealthFactor.into());
    }

    // repaying the borrowed amount :-
    let borrow_transfer_accounts = TransferChecked {
        from: ctx
            .accounts
            .liquidator_borrowed_token_account
            .to_account_info(),
        mint: ctx.accounts.borrowed_mint.to_account_info(),
        to: ctx.accounts.borrowed_bank_token_account.to_account_info(),
        authority: ctx.accounts.liquidator.to_account_info(),
    };

    let borrow_transfer_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        borrow_transfer_accounts,
    );
    let liquidation_amount = total_borrowed
        .checked_mul(borrowed_bank.liquidation_close_factor)
        .unwrap();
    transfer_checked(
        borrow_transfer_cpi_ctx,
        liquidation_amount,
        ctx.accounts.borrowed_mint.decimals,
    )?;

    // transferring the collateral asset to the liquidator token account :-

    let liquidator_amount_with_bonus =
        liquidation_amount + (liquidation_amount * collateral_bank.liquidation_bonus);

    let collatoral_transfer_accounts = TransferChecked {
        from: ctx.accounts.collateral_bank_token_account.to_account_info(),
        mint: ctx.accounts.collateral_mint.to_account_info(),
        to: ctx
            .accounts
            .liquidator_collateral_token_account
            .to_account_info(),
        authority: ctx.accounts.collateral_bank_token_account.to_account_info(),
    };

    let collateral_mint_key = ctx.accounts.collateral_mint.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        b"bank_token_account",
        collateral_mint_key.as_ref(),
        &[ctx.bumps.collateral_bank_token_account],
    ]];

    let collateral_cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        collatoral_transfer_accounts,
    )
    .with_signer(signer_seeds);

    transfer_checked(
        collateral_cpi_ctx,
        liquidator_amount_with_bonus,
        ctx.accounts.collateral_mint.decimals,
    )?;

    Ok(())
}
