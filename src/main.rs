mod config;
mod cmd;

use config::{ DEFAULT_RPC_URL, CONCURRENT_TASK_LIMIT };
use futures::stream::{self, StreamExt};
use alloy::primitives::{Address, B256};
use cmd::{Cli, Commands};
use clap::Parser;
use eyre::Result;


#[tokio::main]
pub async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::FindStorageSlot(cmd) => {
            find_storage_slots(
                parse_tokens_str(cmd.tokens), 
                cmd.rpc_url, 
                cmd.fork_rpc_url,
                cmd.unformatted,
            ).await
        },
        Commands::SetBalance(cmd) => {
            set_balance(
                parse_token_str(&cmd.token)?, 
                parse_token_str(&cmd.holder)?, 
                cmd.target_balance, 
                cmd.rpc_url, 
                cmd.verbose,
            ).await
        }
    }
}

async fn find_storage_slots(
    tokens: Vec<Address>,
    rpc_url: Option<String>,
    fork_rpc_url: Option<String>,
    unformatted_output: bool,
) -> Result<()> {
    fn handle_output(token: Address, res: Result<(Address, B256, f64, String)>, unformatted_output: bool) {
        match res {
            Ok((contract, slot, update_ratio, lang)) => {
                if unformatted_output {
                    println!("{token:?},{contract:?},{slot:?},{update_ratio},{lang},");
                } else {
                    println!("Token: {token:?}");
                    println!("Contract: {contract:?}");
                    println!("Slot: {slot:?}");
                    println!("Update ratio: {update_ratio}");
                    println!("Language: {lang}");
                    println!();
                }
            },
            Err(e) => {
                if unformatted_output {
                    println!("{token:?},,,,,Error: {e:}");
                } else {
                    println!("Token: {token:?}");
                    println!("Error: {e:?}");
                }
            },
        }
    }

    let (rpc_url, _anvil) = if let Some(fork_rpc_url) = fork_rpc_url {
        let anvil = erc20_topup::utils::spawn_anvil(Some(&fork_rpc_url));
        (anvil.endpoint(), Some(anvil))
    } else {
        (rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string()), None)
    };

    // todo: this can get messy as two threads are modifying storage of the same fork instance
    // todo: use tokio instead to manage tasks better  
    let tasks = stream::iter(tokens).map(|token| {
        let rpc_url = rpc_url.clone();
        async move {
            let res = erc20_topup::find_slot(&rpc_url, token, None).await;
            (token, res)
        }
    });

    tasks.buffer_unordered(CONCURRENT_TASK_LIMIT)
        .for_each(|(token, res)| {
            handle_output(token, res, unformatted_output);
            futures::future::ready(())
        }).await;

    Ok(())
}

async fn set_balance(
    token: Address, 
    holder: Address, 
    target_balance: f64,
    rpc_url: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("Setting balance for token {token:?} and holder {holder:?} to {target_balance}");
    }
    let rpc_url = rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string());
    let resulting_bal = erc20_topup::set_balance(
        &rpc_url, 
        token, 
        holder, 
        target_balance, 
        None
    ).await?;
    if verbose {
        println!("New balance: {}", resulting_bal);
    }
    Ok(())
}

fn parse_tokens_str(tokens_str: String) -> Vec<Address> {
    tokens_str
        .split(",")
        .filter_map(|s| parse_token_str(s).ok())
        .collect()
}

fn parse_token_str(token_str: &str) -> Result<Address> {
    let token = token_str.trim().parse::<Address>()?;
    Ok(token)
}