use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient Funds")]
    InsufficientFunds,

    #[msg("Not eligible to borrow this much amount")]
    OverBorrowableAmount,

    #[msg("Over repay amount")]
    OverRepayAmount,

    #[msg("The account does not fall below the health factor, so can't be liquidated")]
    DoesNotFallBelowHealthFactor,
}
