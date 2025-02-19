use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient Funds")]
    InsufficientFunds,

    #[msg("Not eligible to borrow this much amount")]
    OverBorrowableAmount
}
