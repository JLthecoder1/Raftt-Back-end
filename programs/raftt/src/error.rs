use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Offering target amount must be greater than zero")]
    InvalidTargetAmount,
    #[msg("Only the counter authority can update this counter")]
    Unauthorized,
    #[msg("Counter has reached the maximum value")]
    CounterOverflow,
}
