use std::str::FromStr;

use alloy_primitives::{Address, B256};
use clap::{command, Parser};
use clap_derive::{Parser, Subcommand};
use palantiri::{rpc::RpcClient, HttpTransport};
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
enum Commands {
    Block {
        number: u64,
    },
    Balance {
        address: String,
        #[arg(long, default_value = "latest")]
        block: String,
    },
    Tx {
        hash: String,
    },
    Logs {
        from: u64,
        to: u64,
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        topics: Option<Vec<B256>>,
    },
    Code {
        address: String,
        #[arg(long, default_value = "latest")]
        block: String,
    },
    Storage {
        address: String,
        slot: String,
        #[arg(long, default_value = "latest")]
        block: String,
    },
    Receipt {
        hash: String,
    },
    BlockReceipts {
        number: u64,
    },
}

#[derive(Deserialize, Serialize, Clone, clap_derive::ValueEnum, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
    Table,
}

#[derive(Deserialize, Serialize, Default)]
struct Config {
    rpc_url: String,
    default_format: OutputFormat,
}

#[derive(Parser)]
#[command(name = "palantiri")]
struct Cli {
    #[arg(long, env = "ETH_RPC_URL")]
    rpc_url: String,

    #[arg(long)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "text")]
    format: OutputFormat,
}

async fn execute(
    client: RpcClient,
    cmd: Commands,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match cmd {
        Commands::Block { number } => Ok(serde_json::to_value(
            client.get_block_by_number(number, false).await?,
        )?),
        Commands::Balance { address, block } => {
            let address = Address::from_str(&address)?;
            Ok(serde_json::to_value(
                client.get_balance(address, &block).await?,
            )?)
        }
        Commands::Tx { hash } => {
            let hash = B256::from_str(&hash)?;
            Ok(serde_json::to_value(
                client.get_transaction_by_tx_hash(hash).await?,
            )?)
        }
        Commands::Logs {
            from,
            to,
            address,
            topics,
        } => {
            let addr = address.map(|a| Address::from_str(&a).unwrap());
            Ok(serde_json::to_value(
                client.get_logs(from, to, addr, topics).await?,
            )?)
        }
        Commands::Code { address, block } => {
            let addr = Address::from_str(&address)?;
            Ok(serde_json::to_value(client.get_code(addr, block).await?)?)
        }
        Commands::Storage {
            address,
            slot,
            block,
        } => {
            let addr = Address::from_str(&address)?;
            let slot: alloy_primitives::FixedBytes<32> = B256::from_str(&slot)?;
            Ok(serde_json::to_value(client.get_storage_at(addr, slot, block).await?)?)
        }
        Commands::Receipt { hash } => {
            let hash = B256::from_str(&hash)?;
            Ok(serde_json::to_value(client.get_transaction_receipt(hash).await?)?)
        }
        Commands::BlockReceipts { number } => {
            Ok(serde_json::to_value(client.get_block_receipts(number).await?)?)
        }
    }
}

impl Config {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/palantir/config.toml");
        let config_str = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_str)?)
    }
}

fn format_output<T: Serialize + std::fmt::Debug>(
    value: &T,
    format: OutputFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(value)?),
        OutputFormat::Text => Ok(format!("{:#?}", value)),
        OutputFormat::Table => {
            let table = comfy_table::Table::new();
            Ok(table.to_string())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let config = if let Some(path) = cli.config {
        let config_str = std::fs::read_to_string(path)?;
        toml::from_str(&config_str)?
    } else {
        Config::load().unwrap_or_default()
    };

    let url = if cli.rpc_url.is_empty() {
        config.rpc_url
    } else {
        cli.rpc_url
    };

    let client = RpcClient::new(HttpTransport::new(url));

    let result = execute(client, cli.command).await?;
    println!("{}", format_output(&result, cli.format)?);

    Ok(())
}
