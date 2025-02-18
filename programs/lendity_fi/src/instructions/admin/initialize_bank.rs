use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{Bank, ANCHOR_DISCRIMINATOR};

#[derive(Accounts)]
pub struct InitializeBank<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
    init,
    payer = signer,
    space = ANCHOR_DISCRIMINATOR + Bank::INIT_SPACE,
    seeds = [mint.key().as_ref()],
    bump,
  )]
    pub bank: Account<'info, Bank>,

    #[account(
    init,
    token::mint = mint,
    token::authority = bank_token_account,
    token::token_program = token_program,
    payer = signer,
    seeds = [b"bank_token_account", mint.key().as_ref()],
    bump
  )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn initialize_bank_handler(
    ctx: Context<InitializeBank>,
    liquidation_threshold: u64,
    max_ltv: u64,
) -> Result<()> {
    let bank = &mut ctx.accounts.bank;
    bank.authority = ctx.accounts.signer.key();
    bank.mint_address = ctx.accounts.mint.key();
    bank.max_ltv = max_ltv;
    bank.liquidation_threshold = liquidation_threshold;
    Ok(())
}
