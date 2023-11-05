mod cmd;

use clap::Parser;
use cmd::{Cli, Commands};
use eyre::Result;
use ethers::types::H160; // ? Belongs here?

const DEFAULT_RPC_URL: &str = "http://localhost:8545";

#[tokio::main]
pub async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::FindStorageSlot(cmd) => {
            find_storage_slots(
                parse_tokens_str(cmd.tokens), 
                cmd.rpc_url, 
                cmd.fork_rpc_url, 
                cmd.cache
            ).await
        },
        Commands::SetBalance(cmd) => {
            set_balance(
                parse_token_str(&cmd.token)?, 
                parse_token_str(&cmd.holder)?, 
                cmd.target_balance, 
                cmd.rpc_url
            ).await
        }
    }
}

async fn find_storage_slots(
    tokens: Vec<H160>,
    rpc_url: Option<String>,
    fork_rpc_url: Option<String>,
    cache: Option<String>,
) -> Result<()> {
    // ? handle cache in lib.rs?
    // todo: load cache and check if results are already there

    // ! Anvil should not be dropped until all handlers are finished using it
    let (rpc_url, _anvil) = if let Some(fork_rpc_url) = fork_rpc_url {
        let anvil = erc20_topup::utils::spawn_anvil(Some(&fork_rpc_url));
        (anvil.endpoint(), Some(anvil))
    } else {
        (rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string()), None)
    };

    // todo: create a stream and a loading bar
    let mut handlers = Vec::new();
    for token in tokens {
        let rpc_url = rpc_url.clone();
        let handler = tokio::spawn(async move {
            let res = erc20_topup::find_slot(rpc_url, token, None).await;
            (token, res)
        });
        handlers.push(handler);
    }
    let results = futures::future::join_all(handlers).await
        .into_iter()
        .map(|e| {
            match e {
                Ok(e) => e, 
                Err(e) => (H160::zero(), Err(eyre::eyre!(e)))
            }
        })
        .collect::<Vec<_>>();
    
    let mut str_out = String::new();
    for (token, tkn_strg_bal_info) in results {
        match tkn_strg_bal_info {
            Ok((contract, slot, update_ratio, lang)) => {
                str_out.push_str(&format!("{token:?},{contract:?},{slot:?},{update_ratio},{lang},\n"));
            },
            Err(e) => {
                str_out.push_str(&format!("{token:?},,,,,Error: {e:?}\n"));
            },
        }
    }
    println!("{str_out}");

    // todo: write results to cache 

    // output results to stdout in json/csv format


    Ok(())
}

async fn set_balance(
    token: H160, 
    holder: H160, 
    target_balance: f64,
    rpc_url: Option<String> 
) -> Result<()> {
    // todo: load/write cache
    println!("Setting balance for token {token:?} and holder {holder:?} to {target_balance}");
    let rpc_url = rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string());
    let resulting_bal = erc20_topup::set_balance(
        rpc_url, 
        token, 
        holder, 
        target_balance, 
        None
    ).await?;
    println!("New balance: {}", resulting_bal);
    Ok(())
}

fn parse_tokens_str(tokens_str: String) -> Vec<H160> {
    tokens_str
        .split(",")
        .filter_map(|s| parse_token_str(s).ok())
        .collect()
}

fn parse_token_str(token_str: &str) -> Result<H160> {
    let token = token_str.trim().parse::<H160>()?;
    Ok(token)
}