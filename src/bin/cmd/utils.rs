use alloy::{
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, B256},
};
use eyre::Result;


pub fn parse_tokens_str(tokens_str: String) -> Result<Vec<Address>> {
    tokens_str
    .split(",")
    .map(|s| parse_token_str(s))
    .try_fold(Vec::new(), |mut acc, token| {
       token.map(|t| { acc.push(t); acc } )
    })
}

pub fn parse_token_str(token_str: &str) -> Result<Address> {
    let token = token_str.trim().parse::<Address>()?;
    Ok(token)
}

pub fn spawn_anvil(fork_url: Option<&str>) -> AnvilInstance {
    (match fork_url {
        Some(url) => Anvil::new().fork(url),
        None => Anvil::new(),
    }).spawn()
}

pub fn format_find_slot_out(token: Address, res: Result<(Address, B256, f64, String)>, unformatted_output: bool) {
    match res {
        Result::Ok((contract, slot, update_ratio, lang)) => {
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