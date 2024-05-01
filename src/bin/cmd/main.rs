mod balance_setter;
mod config;
mod utils;
mod cmd;

use cmd::{Cli, Commands};
use tokio::task::JoinSet;
use clap::Parser;
use eyre::Result;
use config::DEFAULT_RPC_URL;


#[tokio::main]
pub async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::FindStorageSlot(cmd) => {
            find_storage_slots(
                cmd.tokens, 
                cmd.rpc_url, 
                cmd.fork_rpc_url,
                cmd.unformatted,
            ).await
        },
        Commands::SetBalance(cmd) => {
            set_balance(
                cmd.token, 
                cmd.holder, 
                cmd.target_balance, 
                cmd.rpc_url, 
                cmd.verbose,
            ).await
        }
    }
}

async fn find_storage_slots(
    tokens: String,
    rpc_url: Option<String>,
    fork_rpc_url: Option<String>,
    unformatted_output: bool,
) -> Result<()> {
    let tokens = utils::parse_tokens_str(tokens)?;

    let (rpc_url, _anvil) = 
        if let Some(fork_rpc_url) = fork_rpc_url {
            let anvil = utils::spawn_anvil(Some(&fork_rpc_url));
            (anvil.endpoint(), Some(anvil))
        } else {
            (rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string()), None)
        };

    // todo: consider rpc limit
    let mut task_set = tokens.into_iter().map(|token| {
        let rpc_url = rpc_url.clone();
        async move {
            (token, erc20_topup::find_slot(&rpc_url, token, None).await)
        }
    }).collect::<JoinSet<_>>();

    while let Some(res) = task_set.join_next().await {
        let (token, res) = res?;
        utils::format_find_slot_out(token, res, unformatted_output);
    }

    Ok(())
}

async fn set_balance(
    token: String, 
    holder: String, 
    target_balance: f64,
    rpc_url: Option<String>,
    verbose: bool,
) -> Result<()> {
    let token = utils::parse_token_str(&token)?;
    let holder = utils::parse_token_str(&holder)?;

    if verbose {
        println!("Setting balance for token {token:?} and holder {holder:?} to {target_balance}");
    }
    let rpc_url = rpc_url.unwrap_or(DEFAULT_RPC_URL.to_string());
    let resulting_bal = balance_setter::set_balance(
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

