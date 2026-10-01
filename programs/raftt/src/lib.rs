pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85");

#[program]
pub mod raftt {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, target_amount: u64) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, target_amount)
    }

    pub fn invest(ctx: Context<Invest>, amount: u64) -> Result<()> {
        crate::instructions::invest::handle_invest(ctx, amount)
    }
}
