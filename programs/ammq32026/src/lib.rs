pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
use anchor_lang::prelude::*;
pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("8BPAo6Z8BgMQ4Dr3vErasi8DgUgJB4wuXFzvS2M1rs5z");

#[program]
pub mod amm {

use super::*;

    pub fn initialize( ctx: Context<Initialize>, seed : u64, fees :u16, authority : Option<Pubkey> ) -> Result<()> {
        ctx.accounts.init(seed, fees, authority, ctx.bumps)
    }

    pub fn deposite( ctx: Context <Deposite>,  amount : u64, max_x : u64 , max_y: u64 ) -> Result<()> {
        ctx.accounts.deposite(amount, max_x, max_y)
    }

    pub fn swap(ctx: Context<Swap>, is_x: bool, amount_in: u64, min_amount_out: u64) -> Result<()> {
        ctx.accounts.swap(is_x, amount_in, min_amount_out)
    }

    pub fn withdraw( ctx: Context <Withdraw>, amount : u64, max_x : u64 , max_y: u64 ) -> Result<()> {
        ctx.accounts.withdraw(amount, max_x, max_y)
    }


}

