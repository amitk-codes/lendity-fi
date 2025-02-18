use anchor_lang::prelude::*;

use crate::{User, ANCHOR_DISCRIMINATOR};

#[derive(Accounts)]
pub struct InitializeUser<'info>{
  #[account(mut)]
  pub signer: Signer<'info>,

  #[account(
    init,
    payer = signer,
    space = ANCHOR_DISCRIMINATOR + User::INIT_SPACE,
    seeds = [signer.key().as_ref()],
    bump,
  )]
  pub user_account: Account<'info, User>,

  pub system_program: Program<'info, System>,

}

pub fn initialize_user_handler(ctx: Context<InitializeUser>, usdc_address: u64) -> Result<()>{
  let user_account = &mut ctx.accounts.user_account;
  user_account.owner = ctx.accounts.signer.key();
  user_account.usdc_address = usdc_address;
  Ok(())
}