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
            find_storage_slots(parse_tokens_str(cmd.tokens), cmd.rpc_url, cmd.fork_rpc_url, cmd.cache).await
        },
    }
}


async fn find_storage_slots(
    tokens: Vec<H160>,
    rpc_url: Option<String>,
    fork_rpc_url: Option<String>,
    cache: Option<String>,
) -> Result<()> {

    // todo: load cache and check if results are already there

    // ! Anvil should not be dropped until all handlers are finished using it
    let (rpc_url, _anvil) = if let Some(fork_rpc_url) = fork_rpc_url {
        let anvil = erc20_topup::utils::spawn_anvil(Some(&fork_rpc_url));
        (anvil.endpoint(), Some(anvil))
    } else {
        (rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string()), None)
    };

    let mut handlers = Vec::new();
    for token in tokens {
        let rpc_url = rpc_url.clone();
        let handler = tokio::spawn(async move {
            let res = erc20_topup::find_slot(rpc_url, token, None).await;
            (token, res)
        });
        handlers.push(handler);
    }
    // todo: split handlers in chunks (handler_count / threads) + loading bar
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
            Ok((contract, slot, update_ratio)) => {
                str_out.push_str(&format!("{token:?},{contract:?},{slot:?},{update_ratio},\n"));
            },
            Err(e) => {
                str_out.push_str(&format!("{token:?},,,,Error: {e:?}\n"));
            },
        }
    }
    println!("{str_out}");

    // todo: write results to cache 

    // output results to stdout in json/csv format


    Ok(())
}


fn parse_tokens_str(tokens_str: String) -> Vec<H160> {
    tokens_str
        .split(",")
        .filter_map(|s| s.trim().parse::<H160>().ok())
        .collect()
}