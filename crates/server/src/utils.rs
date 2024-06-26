use alloy::node_bindings::{Anvil, AnvilInstance};
use crate::config::AnvilConfig;


pub fn spawn_anvil(
    fork_url: Option<&str>, 
    config: Option<&AnvilConfig>, 
    fork_optimism: Option<bool>,
) -> AnvilInstance {
    let fork_optimism = fork_optimism.unwrap_or(false);
    let mut anvil = match fork_url {
        Some(url) => Anvil::new().fork(url),
        None => Anvil::new(),
    };
    if let Some(config) = config {
        if let Some(cpu_per_sec) = config.cpu_per_sec {
            anvil = anvil.args(vec![
                "--compute-units-per-second",
                &cpu_per_sec.to_string(),
            ]);
        }
        if let Some(memory_limit) = config.memory_limit {
            anvil = anvil.args(vec![
                "--memory-limit",
                &memory_limit.to_string(),
            ]);
        }
        if let Some(timeout) = config.timeout {
            anvil = anvil.args(vec![
                "--timeout",
                &timeout.to_string(),
            ]);
        }
    }
    if fork_optimism {
        anvil = anvil.args(vec!["--optimism"]);
    }

    anvil.spawn()
}
