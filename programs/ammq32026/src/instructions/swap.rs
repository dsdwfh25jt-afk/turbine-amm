use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer , TransferChecked , transfer_checked}
};
use constant_product_curve::ConstantProduct;

use crate::{error::AmmError, state::Config};

#[derive(Accounts)]

pub struct Swap <'info> {
    #[account(mut)]
    pub user : Signer<'info>,
    pub mint_x : Account<'info, Mint>,
    pub mint_y : Account<'info, Mint>,
     #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config : Account<'info, Config>,
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x_ata: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y_ata: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount_in: u64, min_amount_out: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require_gt!(amount_in, 0, AmmError::InvalidAmount);

        // 1. Identify reserves and decimals based on swap direction
        let (reserve_in, reserve_out) = if is_x {
            (self.vault_x_ata.amount, self.vault_y_ata.amount)
        } else {
            (self.vault_y_ata.amount, self.vault_x_ata.amount)
        };

        require!(reserve_in > 0 && reserve_out > 0, AmmError::EmptyPool);

        // 2. Calculate fee and output amount using u128 to avoid overflow
        // Assuming `config.fee` is in basis points (e.g., 30 = 0.30% out of 10_000)
        let fee_bps = self.config.fee as u128;
        let amount_in_u128 = amount_in as u128;
        let reserve_in_u128 = reserve_in as u128;
        let reserve_out_u128 = reserve_out as u128;

        // Deduct fee from input (rounds fee up in favor of the pool)
        let amount_in_with_fee = amount_in_u128
            .checked_mul(10_000 - fee_bps)
            .ok_or(AmmError::Overflow)?;

        let numerator = amount_in_with_fee
            .checked_mul(reserve_out_u128)
            .ok_or(AmmError::Overflow)?;

        let denominator = reserve_in_u128
            .checked_mul(10_000)
            .ok_or(AmmError::Overflow)?
            .checked_add(amount_in_with_fee)
            .ok_or(AmmError::Overflow)?;

        let amount_out = u64::try_from(numerator / denominator)
            .map_err(|_| AmmError::Overflow)?;

        // 3. Enforce slippage protection and ensure vault isn't drained
        require_gt!(amount_out, 0, AmmError::LiquidityLessThanMinimum);
        require_gte!(amount_out, min_amount_out, AmmError::SlippageExceeded);
        require!(amount_out < reserve_out, AmmError::NoLiquidityInPool);

        // 4. Execute transfers
        self.deposit_tokens(is_x, amount_in)?;
        self.withdraw_tokens(is_x, amount_out)?;

        Ok(())
    }

    fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = if is_x {
            (
                self.user_x_ata.to_account_info(),
                self.vault_x_ata.to_account_info(),
                self.mint_x.to_account_info(),
                self.mint_x.decimals,
            )
        } else {
            (
                self.user_y_ata.to_account_info(),
                self.vault_y_ata.to_account_info(),
                self.mint_y.to_account_info(),
                self.mint_y.decimals,
            )
        };

        let cpi_ctx = CpiContext::new(
            self.token_program.key(),
            TransferChecked {
                from,
                mint,
                to,
                authority: self.user.to_account_info(),
            },
        );

        transfer_checked(cpi_ctx, amount, decimals)
    }

    fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to, mint, decimals) = if is_x {
            (
                self.vault_y_ata.to_account_info(),
                self.user_y_ata.to_account_info(),
                self.mint_y.to_account_info(),
                self.mint_y.decimals,
            )
        } else {
            (
                self.vault_x_ata.to_account_info(),
                self.user_x_ata.to_account_info(),
                self.mint_x.to_account_info(),
                self.mint_x.decimals,
            )
        };

        let seed_bytes = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            seed_bytes.to_le_bytes().as_ref(),
            &[self.config.config_bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.key(),
            TransferChecked {
                from,
                mint,
                to,
                authority: self.config.to_account_info(),
            },
            signer_seeds,
        );

        transfer_checked(cpi_ctx, amount, decimals)
    }
}
