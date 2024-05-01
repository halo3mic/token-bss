use clap::{Parser, Subcommand, Args};


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Find storage slot for a token")]
    FindStorageSlot(FindStorageSlotArgs),
    SetBalance(SetBalanceArgs)
}

#[derive(Args, Debug)]
pub struct FindStorageSlotArgs {
    #[arg(required = true, help = "Comma seperated token addresses.")]
    pub tokens: String,
    #[arg(long, help = "Set Anvil RPC endpoint. Default is http://localhost:8545.")]
    pub rpc_url: Option<String>,
    #[arg(long, help = "Set provider url to be forked. Default: None.")]
    pub fork_rpc_url: Option<String>,
    #[arg(long, help = "True for unformatted output. Default: false.", default_value_t = false)]
    pub unformatted: bool,
}

#[derive(Args, Debug)]
pub struct SetBalanceArgs {
    #[arg(required = true, help = "Address of the token to set balance for.")]
    pub token: String, 
    #[arg(required = true, help = "Address of the holder to set balance for.")]
    pub holder: String,
    #[arg(required = true, help = "Target balance in decimal representation.")]
    pub target_balance: f64,
    #[arg(long, help = "Set Anvil RPC endpoint. Default is http://localhost:8545.")]
    pub rpc_url: Option<String>,
    #[arg(long, help = "True for verbose output. Default: false.", default_value_t = false)]
    pub verbose: bool,
}

