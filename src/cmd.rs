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
    // todo: specify holder address
    #[command(about = "Find storage slot for a token")]
    FindStorageSlot(FindStorageSlot),

}

#[derive(Args, Debug)]
pub struct FindStorageSlot {
    #[arg(required = true, help = "Comma seperated token addresses.")]
    pub tokens: String,
    #[arg(long, help = "Set provider endpoint. Default: http://localhost:8545.")]
    pub rpc_url: Option<String>,
    #[arg(long, help = "Set fork provider endpoint. Default: None.")]
    pub fork_rpc_url: Option<String>,
    #[arg(long, help = "Cache file. If not specified, use default cache file.")]
    pub cache: Option<String>,
}

