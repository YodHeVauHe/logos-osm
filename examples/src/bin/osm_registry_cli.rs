use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use clap::{Parser, Subcommand};
use lee_core::{account::AccountId, program::PdaSeed};
use osm_registry_core::{RegionEntry, Registry};
use wallet::{
    config::{SequencerConnectionData, WalletConfigOverrides},
    AccountIdentity, WalletCore,
};

#[derive(Parser)]
#[command(about = "Submit OSM registry transactions to a LEZ v0.3.1 sequencer")]
struct Args {
    #[arg(long, env = "OSM_RPC_URL")]
    rpc_url: Option<String>,
    #[arg(long, env = "OSM_PROGRAM_ID")]
    program_id: String,
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
    let wallet = wallet(&args.rpc_url)?;
    let seed = PdaSeed::new({
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(b"registry");
        bytes
    });
    let registry_id = AccountId::for_public_pda(&program_id, &seed);

    match args.command {
        Command::Get { raw } => {
            let account = wallet.get_account_public(registry_id).await?;
            let bytes = account.data.shard(program_id);
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

fn wallet(rpc_url: &Option<String>) -> anyhow::Result<WalletCore> {
    if let Some(rpc_url) = rpc_url {
        let config = wallet::helperfunctions::fetch_config_path()?;
        let storage = wallet::helperfunctions::fetch_persistent_storage_path()?;
        let overrides = WalletConfigOverrides {
            sequencer: Some(SequencerConnectionData {
                sequencer_addr: rpc_url.parse()?,
                basic_auth: None,
            }),
            ..Default::default()
        };
        WalletCore::new_update_chain(config, storage, Some(overrides))
    } else {
        WalletCore::from_env()
    }
}

async fn send(
    wallet: &WalletCore,
    program_id: AccountId,
    registry_id: AccountId,
    payer: &str,
    instruction: &Instruction,
) -> anyhow::Result<()> {
    let payer = AccountId::from_str(payer)?;
    let account = AccountIdentity::PublicNoSign(registry_id).select_program_shard(program_id);
    let hash = wallet
        .send_pub_tx_paid_by(
            vec![account],
            borsh::to_vec(instruction)?,
            program_id,
            Some(payer),
        )
        .await?;
    wallet.poll_transaction(hash).await?;
    println!("Transaction: {hash:?}");
    Ok(())
}
