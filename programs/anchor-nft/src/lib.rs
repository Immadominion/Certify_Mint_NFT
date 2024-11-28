use anchor_lang::prelude::*;
use anchor_lang::system_program::ID;
use mpl_core::{self, instructions::CreateV1CpiBuilder, types::PluginAuthorityPair};

mod constants;
mod error;
mod utils;

use crate::{constants::AUTHORITY_SEED, error::CandyError};

declare_id!("F6ziWNy3jFrRnzhrNrNRGTEYghHpZXbpyfTBwdMZJHNH");

#[derive(Accounts)]
pub struct MintAsset<'info> {
    /// CHECK: This is safe
    pub authority_pda: AccountInfo<'info>,
    /// CHECK: This is safe
    pub payer: AccountInfo<'info>,
    /// CHECK: This is safe
    pub asset_owner: AccountInfo<'info>,
    /// CHECK: This is safe
    pub asset: AccountInfo<'info>,
    /// CHECK: This is safe
    pub collection: AccountInfo<'info>,
    /// CHECK: This is safe
    pub mpl_core_program: AccountInfo<'info>,
    /// CHECK: This is safe
    pub system_program: AccountInfo<'info>,
    /// CHECK: This is safe
    pub sysvar_instructions: Option<AccountInfo<'info>>,
    /// CHECK: This is safe
    pub recent_slothashes: AccountInfo<'info>,
    /// CHECK: This is safe
    #[account(mut)]
    pub candy_machine: Box<Account<'info, CandyMachine>>,
}

pub struct MintAccounts<'info> {
    pub authority_pda: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub asset_owner: AccountInfo<'info>,
    pub asset: AccountInfo<'info>,
    pub collection: AccountInfo<'info>,
    pub mpl_core_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub sysvar_instructions: Option<AccountInfo<'info>>,
    pub recent_slothashes: AccountInfo<'info>,
}

#[account]
pub struct CandyMachine {
    pub items_redeemed: u64,
    pub data: CandyMachineData,
    pub collection_mint: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CandyMachineData {
    pub items_available: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ConfigLine {
    pub name: String,
    pub uri: String,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MintAssetArgs {
    pub plugins: Vec<PluginAuthorityPair>,
    pub name: String,
    pub uri: String,
}

pub fn mint_asset<'info>(
    ctx: Context<'_, '_, '_, 'info, MintAsset<'info>>,
    mint_args: MintAssetArgs,
) -> Result<()> {
    let accounts = MintAccounts {
        authority_pda: ctx.accounts.authority_pda.to_account_info(),
        collection: ctx.accounts.collection.to_account_info(),
        asset_owner: ctx.accounts.asset_owner.to_account_info(),
        asset: ctx.accounts.asset.to_account_info(),
        payer: ctx.accounts.payer.to_account_info(),
        recent_slothashes: ctx.accounts.recent_slothashes.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        mpl_core_program: ctx.accounts.mpl_core_program.to_account_info(),
        sysvar_instructions: ctx
            .accounts
            .sysvar_instructions
            .as_ref()
            .map(|a| a.to_account_info()),
    };

    process_mint_asset(
        &mut ctx.accounts.candy_machine,
        accounts,
        ctx.bumps["authority_pda"],
        &mint_args,
    )
}

pub(crate) fn process_mint_asset(
    candy_machine: &mut Box<Account<'_, CandyMachine>>,
    accounts: MintAccounts,
    bump: u8,
    mint_args: &MintAssetArgs,
) -> Result<()> {
    if !accounts.asset.data_is_empty() {
        return err!(CandyError::MetadataAccountMustBeEmpty);
    }

    if candy_machine.items_redeemed >= candy_machine.data.items_available {
        return err!(CandyError::CandyMachineEmpty);
    }

    let config_line = ConfigLine {
        name: mint_args.name.clone(),
        uri: mint_args.uri.clone(),
    };

    candy_machine.items_redeemed = candy_machine
        .items_redeemed
        .checked_add(1)
        .ok_or(CandyError::NumericalOverflowError)?;

    create_and_mint(
        candy_machine,
        accounts,
        bump,
        config_line,
        &mint_args.plugins,
    )
}

pub fn get_config_line(// candy_machine: &Account<'_, CandyMachine>,
    // index: usize,
    // mint_number: u64,
) -> Result<ConfigLine> {
    Ok(ConfigLine {
        name: "NFT Name".to_string(),
        uri: "https://example.com/nft-metadata.json".to_string(),
    })
}

fn create_and_mint(
    candy_machine: &mut Box<Account<'_, CandyMachine>>,
    accounts: MintAccounts,
    bump: u8,
    config_line: ConfigLine,
    plugins: &[PluginAuthorityPair],
) -> Result<()> {
    let candy_machine_key = candy_machine.key();
    let authority_seeds = [
        AUTHORITY_SEED.as_bytes(),
        candy_machine_key.as_ref(),
        &[bump],
    ];

    CreateV1CpiBuilder::new(&accounts.mpl_core_program)
        .payer(&accounts.payer)
        .asset(&accounts.asset)
        .owner(Some(&accounts.asset_owner))
        .name(config_line.name) // Dynamic name
        .uri(config_line.uri) // Dynamic URI
        .collection(Some(&accounts.collection))
        .plugins(plugins.to_vec())
        .data_state(mpl_core::types::DataState::AccountState)
        .authority(Some(&accounts.authority_pda))
        .system_program(&accounts.system_program)
        .invoke_signed(&[&authority_seeds])
        .map_err(|error| error.into())
}
