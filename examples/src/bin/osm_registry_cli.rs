use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use clap::{Parser, Subcommand};
use lee_core::{account::AccountId, program::PdaSeed};
use osm_registry_core::{RegionEntry, Registry};
use sequencer_service_rpc::{RpcClient as _, SequencerClient};
use wallet::{
    config::{SequencerConnectionData, WalletConfigOverrides},
    AccountIdentity, WalletCore,
};

/// Fixed per-transaction wire overhead (header, selectors, nonces, fee
/// declaration, one signature and key) on top of the instruction bytes, rounded
/// up so the estimate never undershoots the sequencer's `data_bytes`.
const TX_OVERHEAD_BYTES: u64 = 1_024;

/// The fee reserve the sequencer screens a public transaction against before
/// admission (`lez/programs/fee/core/src/assess.rs::fee_reserve`):
/// `gas_limit * base_fee_exec + data_bytes * base_fee_stor + tip`, priced here at
/// the next block's ceiling so a fee step between quote and inclusion still fits.
fn fee_reserve_estimate(
    gas_limit: u64,
    data_bytes: u64,
    base_fee_exec: u64,
    base_fee_stor: u64,
) -> u128 {
    u128::from(gas_limit) * u128::from(base_fee_exec)
        + u128::from(data_bytes) * u128::from(base_fee_stor)
}

/// Refuses to submit when the payer cannot cover the reserve, naming the
/// numbers instead of surfacing the sequencer's bare "Incorrect fee".
fn check_payer_can_fund(balance: u128, reserve: u128, gas_limit: u64) -> anyhow::Result<()> {
    if balance < reserve {
        anyhow::bail!(
            "fee payer balance {balance} is below the fee reserve {reserve} \
             (gas limit {gas_limit}); the sequencer would reject this as \"Incorrect fee\". \
             Lower --gas-limit / OSM_GAS_LIMIT or fund the payer"
        );
    }
    Ok(())
}

#[derive(Parser)]
#[command(about = "Submit OSM registry transactions to a LEZ v0.4 sequencer")]
struct Args {
    #[arg(long, env = "OSM_RPC_URL")]
    rpc_url: Option<String>,
    #[arg(long, env = "OSM_PROGRAM_ID")]
    program_id: String,
    /// Execution gas limit declared on each transaction. The sequencer reserves
    /// `gas_limit * base_fee_exec` from the payer up front, so a payer holding
    /// less than that is turned away with "Incorrect fee" even when the actual
    /// cost would fit. Defaults to the wallet config's `gas_limit`.
    #[arg(long, env = "OSM_GAS_LIMIT")]
    gas_limit: Option<u64>,
    /// Blocks to poll for inclusion before giving up (wallet default: 5, which
    /// times out on testnet even when the transaction lands later).
    #[arg(long, env = "OSM_POLL_BLOCKS", default_value_t = 40)]
    poll_blocks: usize,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long, env = "OSM_FEE_PAYER")]
        payer: String,
    },
    Register {
        #[arg(long, env = "OSM_FEE_PAYER")]
        payer: String,
        #[arg(long)]
        entry_json: String,
    },
    Batch {
        #[arg(long, env = "OSM_FEE_PAYER")]
        payer: String,
        #[arg(long)]
        entries_json: String,
    },
    Get {
        #[arg(long)]
        raw: bool,
    },
    Pda,
}

/// The registry PDA seed: `b"registry"` followed by 24 zero bytes.
fn registry_seed() -> PdaSeed {
    let mut bytes = [0; 32];
    bytes[..8].copy_from_slice(b"registry");
    PdaSeed::new(bytes)
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
enum Instruction {
    InitRegistry,
    RegisterRegion(RegionEntry),
    RegisterBatch { entries: Vec<RegionEntry> },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let program_id = AccountId::from_str(&args.program_id)?;
    let wallet = wallet(&args.rpc_url, args.gas_limit, args.poll_blocks)?;
    let registry_id = AccountId::for_public_pda(&program_id, &registry_seed());

    match args.command {
        Command::Get { raw } => {
            // v0.4: the registry is this program's actor state on the PDA.
            let account = wallet.get_account_public(registry_id).await?;
            let bytes: &[u8] = account
                .as_ref()
                .map(|account| account.data.actor_state(program_id).as_ref())
                .unwrap_or_default();
            if bytes.is_empty() {
                println!("null");
            } else if raw {
                println!("{}", hex::encode(bytes));
            } else {
                let registry = Registry::try_from_slice(bytes)?;
                println!("{}", serde_json::to_string_pretty(&registry.entries)?);
            }
        }
        Command::Init { payer } => {
            send(
                &wallet,
                program_id,
                registry_id,
                &payer,
                &Instruction::InitRegistry,
            )
            .await?;
        }
        Command::Register { payer, entry_json } => {
            let entry: RegionEntry = serde_json::from_str(&entry_json)?;
            send(
                &wallet,
                program_id,
                registry_id,
                &payer,
                &Instruction::RegisterRegion(entry),
            )
            .await?;
        }
        Command::Batch {
            payer,
            entries_json,
        } => {
            let entries: Vec<RegionEntry> = serde_json::from_str(&entries_json)?;
            send(
                &wallet,
                program_id,
                registry_id,
                &payer,
                &Instruction::RegisterBatch { entries },
            )
            .await?;
        }
        Command::Pda => println!("{registry_id}"),
    }
    Ok(())
}

fn wallet(
    rpc_url: &Option<String>,
    gas_limit: Option<u64>,
    poll_blocks: usize,
) -> anyhow::Result<WalletCore<SequencerClient>> {
    let config = wallet::utils::fetch_config_path()?;
    let storage = wallet::utils::fetch_persistent_storage_path()?;
    // v0.4 wallets may be password-encrypted (`LEE_WALLET_PASSWORD`).
    let password = wallet::utils::read_password(&storage)?;
    let sequencer = rpc_url
        .as_ref()
        .map(|url| -> anyhow::Result<_> {
            Ok(SequencerConnectionData {
                sequencer_addr: url.parse()?,
                basic_auth: None,
            })
        })
        .transpose()?;
    let overrides = WalletConfigOverrides {
        sequencer,
        gas_limit,
        seq_tx_poll_max_blocks: Some(poll_blocks),
        ..Default::default()
    };
    WalletCore::new_update_chain(config, storage, Some(overrides), &password)
}

async fn send(
    wallet: &WalletCore<SequencerClient>,
    program_id: AccountId,
    registry_id: AccountId,
    payer: &str,
    instruction: &Instruction,
) -> anyhow::Result<()> {
    let payer = AccountId::from_str(payer)?;
    let instruction_data = borsh::to_vec(instruction)?;
    let gas_limit = wallet.config().gas_limit;
    let quote = wallet.client().get_fee_state().await?;
    let data_bytes = u64::try_from(instruction_data.len())? + TX_OVERHEAD_BYTES;
    let reserve = fee_reserve_estimate(
        gas_limit,
        data_bytes,
        quote.next_base_fee_exec_ceiling,
        quote.next_base_fee_stor_ceiling,
    );
    let balance_before = wallet.get_account_balance(payer).await?;
    eprintln!(
        "fee preflight: payer balance {balance_before}, reserve ~{reserve} (gas limit {gas_limit})"
    );
    check_payer_can_fund(balance_before, reserve, gas_limit)?;
    // v0.4 actor model: one message to the registry actor (PDA, program).
    // `PublicPda` attaches the PDA admission evidence a fresh registry account
    // needs; without it the sequencer reverts the call on an unknown account.
    let actor = AccountIdentity::PublicPda {
        program: program_id,
        seed: registry_seed(),
    }
    .select_program_actor_state(program_id);
    debug_assert_eq!(actor.identity.account_id(), registry_id);
    let hash = wallet
        .send_pub_tx_paid_by(vec![actor], 0, instruction_data, Some(payer))
        .await?;
    println!("Submitted: {hash:?}");
    wallet.poll_transaction(hash).await?;
    println!("Transaction: {hash:?}");
    let balance_after = wallet.get_account_balance(payer).await?;
    eprintln!("fee paid: {}", balance_before.saturating_sub(balance_after));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_matches_the_sequencer_formula() {
        // 2,000,000 gas at base fee 8 plus 1,500 bytes at 8.
        assert_eq!(fee_reserve_estimate(2_000_000, 1_500, 8, 8), 16_012_000);
    }

    #[test]
    fn default_gas_limit_is_refused_when_balance_is_below_reserve() {
        // The observed failure: 15,383,554 left, 2M gas at base fee 8.
        let reserve = fee_reserve_estimate(2_000_000, 1_500, 8, 8);
        let err = check_payer_can_fund(15_383_554, reserve, 2_000_000).unwrap_err();
        assert!(err.to_string().contains("below the fee reserve"));
    }

    #[test]
    fn a_lower_gas_limit_fits_the_same_balance() {
        let reserve = fee_reserve_estimate(1_000_000, 1_500, 9, 9);
        check_payer_can_fund(15_383_554, reserve, 1_000_000).unwrap();
    }
}
