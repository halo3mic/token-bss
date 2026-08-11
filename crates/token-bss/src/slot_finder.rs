use futures::future::join_all;
use super::{
    trace_parser::TraceParser, 
    ops::{token, trace}, 
    lang::EvmLanguage, 
    utils,
};
use crate::common::*;

// todo: make a builder here
// todo: use a struct
type SlotOutput = (Address, B256, f64, String);

pub async fn find_balance_slots_and_update_ratio<'a, P, T>(
    provider: &P,
    holder: Address, 
    token: Address,
    trace_fn: Option<TraceFn>,
) -> Result<SlotOutput> 
    where P: Provider<T>, T: Transport + Clone
{
    let slots = find_balance_slots(provider, holder, token, trace_fn).await?;
    closest_balance_slot(provider, token, holder, slots).await
}

pub async fn find_allowance_slots_and_update_ratio<'a, P, T>(
    provider: &P,
    owner: Address,
    spender: Address,
    token: Address,
    trace_fn: Option<TraceFn>,
) -> Result<SlotOutput>
    where P: Provider<T>, T: Transport + Clone
{
    let slots = find_allowance_slots(provider, owner, spender, token, trace_fn).await?;
    closest_allowance_slot(provider, token, owner, spender, slots).await
}

pub async fn find_balance_slots<'a, P, T, N>(
    provider: &P,
    holder: Address,
    token: Address,
    trace_fn: Option<TraceFn>,
) -> Result<Vec<(Address, B256, EvmLanguage)>> 
    where P: Provider<T, N>, T: Transport + Clone, N: Network
{
    let tx_request = token::balanceof_call_req(holder, token)?;
    let response = trace::default_trace_call(provider, tx_request, None, trace_fn).await?;
    let matches = TraceParser::parse(response.struct_logs, token, holder)?;
    Ok(matches)
}

pub async fn find_allowance_slots<'a, P, T, N>(
    provider: &P,
    owner: Address,
    spender: Address,
    token: Address,
    trace_fn: Option<TraceFn>,
) -> Result<Vec<(Address, B256, EvmLanguage)>>
    where P: Provider<T, N>, T: Transport + Clone, N: Network
{
    let tx_request = token::allowance_call_req(owner, spender, token)?;
    let response = trace::default_trace_call(provider, tx_request, None, trace_fn).await?;
    let matches = TraceParser::parse_allowance(response.struct_logs, token, owner, spender)?;
    Ok(matches)
}

// Note this would choose 0 over 2
async fn closest_balance_slot<P, T>(
    provider: &P,
    token: Address,
    holder: Address,
    slots: Vec<(Address, B256, EvmLanguage)>
) -> Result<SlotOutput, eyre::Error> 
    where P: Provider<T>, T: Transport + Clone
{
    let d_one = |x: f64| ((x - 1.0).abs() * 100.) as u8;
    let future_results = join_all(slots.into_iter()
        .map(|(c, s, la)| async move {
            slot_update_to_bal_ratio(provider, token, c,s, holder, la)
                .await
                .map(|ur| (c, s, ur, la.to_string()))
        })
    );
    future_results.await
        .into_iter()
        .filter_map(|x| x.ok())
        .min_by_key(|x| d_one(x.2))
        .ok_or_else(|| eyre::eyre!("No valid slots found"))
}

async fn closest_allowance_slot<P, T>(
    provider: &P,
    token: Address,
    owner: Address,
    spender: Address,
    slots: Vec<(Address, B256, EvmLanguage)>,
) -> Result<SlotOutput, eyre::Error>
    where P: Provider<T>, T: Transport + Clone
{
    let d_one = |x: f64| ((x - 1.0).abs() * 100.) as u8;
    let future_results = join_all(slots.into_iter().map(|(c, s, la)| async move {
        slot_update_to_allowance_ratio(provider, token, c, s, owner, spender, la)
            .await
            .map(|ur| (c, s, ur, la.to_string()))
    }));
    future_results.await
        .into_iter()
        .filter_map(|x| x.ok())
        .min_by_key(|x| d_one(x.2))
        .ok_or_else(|| eyre::eyre!("No valid slots found"))
}

// todo: too many params
// todo: more suiting name
async fn slot_update_to_bal_ratio<P, T>(
    provider: &P, 
    token: Address,
    storage_contract: Address,
    slot: B256,
    holder: Address,
    lang: EvmLanguage,
) -> Result<f64> 
    where P: Provider<T>, T: Transport + Clone
{
    let new_slot_val = U256::from(rand::random::<u128>()); // todo: In scenario where this is excatly the same as the current balance it fails
    let map_loc = lang.mapping_loc(slot, holder);
    let call_request = token::balanceof_call_req(holder, token)?;

    let override_bal_future = token::call_request_with_storage_overrides(
        provider,
        &call_request,
        storage_contract,
        map_loc,
        new_slot_val.into(),
    );
    let real_bal_future = token::call_request(provider, &call_request);
    let (override_bal, real_bal) = tokio::try_join!(override_bal_future, real_bal_future)?;

    if override_bal == real_bal {
        return Err(eyre::eyre!("Balance not updated"));
    }
    let update_ratio = utils::ratio_f64(override_bal, new_slot_val, None);
    
    Ok(update_ratio)
}

async fn slot_update_to_allowance_ratio<P, T>(
    provider: &P,
    token: Address,
    storage_contract: Address,
    slot: B256,
    owner: Address,
    spender: Address,
    lang: EvmLanguage,
) -> Result<f64>
    where P: Provider<T>, T: Transport + Clone
{
    let new_slot_val = U256::from(rand::random::<u128>());
    let map_loc = lang.nested_mapping_loc(slot, owner, spender);
    let call_request = token::allowance_call_req(owner, spender, token)?;

    let override_future = token::call_request_with_storage_overrides(
        provider,
        &call_request,
        storage_contract,
        map_loc,
        new_slot_val.into(),
    );
    let real_future = token::call_request(provider, &call_request);
    let (override_val, real_val) = tokio::try_join!(override_future, real_future)?;

    if override_val == real_val {
        return Err(eyre::eyre!("Allowance not updated"));
    }
    Ok(utils::ratio_f64(override_val, new_slot_val, None))
}


#[cfg(test)]
mod tests {
    use alloy::{node_bindings::{Anvil, AnvilInstance}, providers::ReqwestProvider};
    use super::*;

    pub fn spawn_anvil_provider(fork_url: Option<&str>) -> Result<(ReqwestProvider, AnvilInstance)> {
        spawn_anvil_provider_with_opts(fork_url, false)
    }

    pub fn spawn_anvil_provider_with_opts(
        fork_url: Option<&str>,
        optimism: bool,
    ) -> Result<(ReqwestProvider, AnvilInstance)> {
        let anvil_fork = spawn_anvil_with_opts(fork_url, optimism);
        let provider = ReqwestProvider::new_http(anvil_fork.endpoint().parse()?);
        Ok((provider, anvil_fork))
    }

    pub fn spawn_anvil_with_opts(fork_url: Option<&str>, optimism: bool) -> AnvilInstance {
        let mut anvil = match fork_url {
            Some(url) => Anvil::new().fork(url),
            None => Anvil::new(),
        };
        if optimism {
            anvil = anvil.args(["--optimism"]);
        }
        anvil.spawn()
    }

    async fn anvil_fund_account<P, T>(provider: &P, address: Address) -> Result<()>
        where P: Provider<T>, T: Transport + Clone
    {
        // OP-stack Anvil still charges L1 data gas on eth_call/traceCall even with gas_price=0.
        let _: () = provider
            .client()
            .request("anvil_setBalance", (address, U256::from(10).pow(U256::from(24))))
            .await
            .map_err(|e| eyre::eyre!("anvil_setBalance failed: {e}"))?;
        Ok(())
    } 
    
    pub fn env_var(var: &str) -> Result<String> {
        dotenv::dotenv().ok();
        std::env::var(var).map_err(|_| eyre::eyre!("{} not set", var))
    }

    fn rpc_endpoint() -> Result<String> {
        env_var("ETH_RPC_URL")
    }

    /// Resolve RPC for a chain from `{CHAIN}_RPC`, `{CHAIN}_FORK_RPC`, or `ETH_RPC_URL` (ethereum only).
    fn chain_rpc(chain: &str) -> Option<String> {
        dotenv::dotenv().ok();
        let key = chain.to_uppercase();
        std::env::var(format!("{key}_RPC"))
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                std::env::var(format!("{key}_FORK_RPC"))
                    .ok()
                    .filter(|s| !s.trim().is_empty())
            })
            .or_else(|| {
                if chain.eq_ignore_ascii_case("ethereum") {
                    std::env::var("ETH_RPC_URL").ok().filter(|s| !s.trim().is_empty())
                } else {
                    None
                }
            })
    }

    fn is_op_stack(chain: &str) -> bool {
        matches!(chain.to_ascii_lowercase().as_str(), "optimism" | "base" | "unichain")
    }

    fn format_slot(slot: B256) -> String {
        let n: U256 = slot.into();
        // Prefer compact decimal for classic low slots; hex for ERC-7201 / namespaced slots.
        if n < U256::from(1024u64) {
            format!("{n}")
        } else {
            format!("{n:#x}")
        }
    }

    fn holder() -> Address {
        "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap()
    }

    fn spender() -> Address {
        "0x1111111111111111111111111111111111111111".parse().unwrap()
    }

    async fn assert_balance_and_allowance_slots<P, T>(
        provider: &P,
        token: Address,
        expected_balance_slot: u64,
        expected_allowance_slot: u64,
        expected_balance_contract: Option<Address>,
        balance_ratio_exact: Option<f64>,
    ) -> Result<()>
        where P: Provider<T>, T: Transport + Clone
    {
        let holder = holder();
        let spender = spender();

        let (bal_contract, bal_slot, bal_ratio, _) =
            find_balance_slots_and_update_ratio(provider, holder, token, None).await?;
        assert_eq!(bal_slot, B256::from(U256::from(expected_balance_slot)));
        if let Some(c) = expected_balance_contract {
            assert_eq!(bal_contract, c);
        }
        match balance_ratio_exact {
            Some(r) => assert_eq!(bal_ratio, r),
            None => assert!(bal_ratio > 1.0),
        }

        let (allow_contract, allow_slot, allow_ratio, _) =
            find_allowance_slots_and_update_ratio(provider, holder, spender, token, None).await?;
        assert_eq!(allow_slot, B256::from(U256::from(expected_allowance_slot)));
        assert_eq!(allow_ratio, 1.0);
        if let Some(c) = expected_balance_contract {
            assert_eq!(allow_contract, c);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_slot_finding_snx() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse()?;
        let storage: Address = "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse()?;

        let bal = find_balance_slots(&provider, holder(), token, None).await?;
        assert_eq!(bal.len(), 1);
        assert_eq!(bal[0].0, storage);
        assert_eq!(bal[0].1, B256::from(U256::from(3)));

        let allow = find_allowance_slots(&provider, holder(), spender(), token, None).await?;
        assert!(allow.iter().any(|(c, s, _)| *c == storage && *s == B256::from(U256::from(4))));
        Ok(())
    }

    #[tokio::test]
    async fn test_slots_eth_usdc() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 9, 10, Some(token), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_usdt() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 2, 5, Some(token), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_dai() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 2, 3, Some(token), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_sbtc() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xfE18be6b3Bd88A2D2A7f928d00292E7a9963CfC6".parse()?;
        let storage: Address = "0x4f6296455F8d754c19821cF1EC8FEBF2cD456E67".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 3, 4, Some(storage), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_snx() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F".parse()?;
        let storage: Address = "0x5b1b5fea1b99d83ad479df0c222f0492385381dd".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 3, 4, Some(storage), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_stlink() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xb8b295df2cd735b15BE5Eb419517Aa626fc43cD5".parse()?;
        // Balance update ratio is > 1 (rebasing); allowance maps 1:1.
        assert_balance_and_allowance_slots(&provider, token, 0xfc, 0x34, Some(token), None).await
    }

    #[tokio::test]
    async fn test_slots_eth_crv() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0x6c3f90f043a72fa612cbac8115ee7e52bde6e490".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 3, 4, Some(token), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_basic() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xf25c91c87e0b1fd9b4064af0f427157aab0193a7".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 3, 4, Some(token), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_eurcv() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0x5f7827fdeb7c20b443265fc2f40845b715385ff2".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 140, 141, Some(token), Some(1.0)).await
    }

    #[tokio::test]
    async fn test_slots_eth_yv1inch() -> Result<()> {
        let (provider, _anvil_instance) = spawn_anvil_provider(Some(&rpc_endpoint()?))?;
        let token: Address = "0xB8C3B7A2A618C552C23B1E4701109a9E756Bab67".parse()?;
        assert_balance_and_allowance_slots(&provider, token, 3, 4, Some(token), Some(1.0)).await
    }

    use alloy::network::Ethereum;

    #[tokio::test]
    async fn test_slots_uni_arbitrum() -> Result<()> {
        let rpc_endpoint = "https://arb1.arbitrum.io/rpc";
        let (fork_provider, _anvil_instance) = spawn_anvil_provider(Some(rpc_endpoint))?;
        let provider = ReqwestProvider::<Ethereum>::new_http(rpc_endpoint.parse()?);
        let token: Address = "0xfa7f8980b0f1e64a2062791cc3b0871572f1f7f0".parse()?;
        let holder = holder();
        let spender = spender();

        let result = find_balance_slots(&fork_provider, holder, token, None).await?;
        let (_c, bal_slot, bal_ratio, _) = closest_balance_slot(&provider, token, holder, result).await?;
        assert_eq!(bal_slot, B256::from(U256::from(51)));
        assert_eq!(bal_ratio, 1.0);

        let result = find_allowance_slots(&fork_provider, holder, spender, token, None).await?;
        let (_c, allow_slot, allow_ratio, _) =
            closest_allowance_slot(&provider, token, holder, spender, result).await?;
        assert_eq!(allow_slot, B256::from(U256::from(52)));
        assert_eq!(allow_ratio, 1.0);
        Ok(())
    }

    /// Discovery printout for a multi-chain token matrix.
    ///
    /// Provide RPCs via env (`ETHEREUM_RPC` / `ETHEREUM_FORK_RPC` / `ETH_RPC_URL`, and the same
    /// pattern for `ARBITRUM`, `AVALANCHE`, `BASE`, `POLYGON`, `PLASMA`, `UNICHAIN`), then:
    ///
    /// ```bash
    /// cargo test -p token-bss print_balance_and_allowance_slots_for_token_matrix -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "multi-chain discovery; set chain RPC env vars and run with --ignored --nocapture"]
    async fn print_balance_and_allowance_slots_for_token_matrix() -> Result<()> {
        // (symbol, chain, address)
        let tokens: &[(&str, &str, &str)] = &[
            ("DAI", "ethereum", "0x6b175474e89094c44da98b954eedeac495271d0f"),
            ("EURC", "base", "0x60a3e35cc302bfa44cb288bc5a4f316fdb1adb42"),
            ("EURC", "ethereum", "0x1abaea1f7c830bd89acc67ec4af516284b1bc33c"),
            ("JPYC", "ethereum", "0xe7c3d8c9a439fede00d2600032d5db0be71c3c29"),
            ("LINK", "ethereum", "0x514910771af9ca656af840dff83e8264ecf986ca"),
            ("PYUSD", "ethereum", "0x6c3ea9036406852006290770bedfcaba0e23a0e8"),
            ("USDC", "arbitrum", "0xaf88d065e77c8cc2239327c5edb3a432268e5831"),
            ("USDC", "avalanche", "0xb97ef9ef8734c71904d8002f8b6bc66dd9c48a6e"),
            ("USDT", "arbitrum", "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"),
            ("USDT", "avalanche", "0x9702230a8ea53601f5cd2dc00fdbc13d4df4a8c7"),
            ("USDT", "base", "0x1217bfe6c773eec6cc4a38b5dc45b92292b6e189"),
            ("USDT", "base", "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2"),
            ("USDT", "plasma", "0xb8ce59fc3717ada4c02eadf9682a9e934f625ebb"),
            ("USDT", "polygon", "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"),
            ("USDT", "unichain", "0x9151434b16b9763660705744891fa906f660ecc5"),
            ("WBTC", "arbitrum", "0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f"),
            ("XAUT0", "arbitrum", "0x40461291347e1ecbb09499f3371d3f17f10d7159"),
            ("XAUT0", "avalanche", "0x2775d5105276781b4b85ba6ea6a6653beed1dd32"),
            ("cbBTC", "arbitrum", "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf"),
            ("cbBTC", "base", "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf"),
            ("cbBTC", "ethereum", "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf"),
        ];

        println!("chain | token | address | balance slot | allowance slot");

        let mut by_chain: std::collections::BTreeMap<&str, Vec<(&str, &str)>> =
            std::collections::BTreeMap::new();
        for &(symbol, chain, address) in tokens {
            by_chain.entry(chain).or_default().push((symbol, address));
        }

        let holder = holder();
        let spender = spender();

        for (chain, chain_tokens) in by_chain {
            let Some(rpc) = chain_rpc(chain) else {
                for (symbol, address) in chain_tokens {
                    println!("{chain} | {symbol} | {address} | <no RPC> | <no RPC>");
                }
                continue;
            };

            let (provider, _anvil) =
                match spawn_anvil_provider_with_opts(Some(&rpc), is_op_stack(chain)) {
                    Ok(v) => v,
                    Err(e) => {
                        for (symbol, address) in chain_tokens {
                            println!("{chain} | {symbol} | {address} | <anvil err: {e}> | <anvil err>");
                        }
                        continue;
                    }
                };

            // Fund callers used in view calls (needed on OP-stack Anvil for L1 gas checks).
            if is_op_stack(chain) {
                let _ = anvil_fund_account(&provider, holder).await;
                let _ = anvil_fund_account(&provider, spender).await;
            }

            for (symbol, address) in chain_tokens {
                let token: Address = match address.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        println!("{chain} | {symbol} | {address} | <bad addr: {e}> | <bad addr>");
                        continue;
                    }
                };

                let bal_slot = match find_balance_slots_and_update_ratio(
                    &provider, holder, token, None,
                ).await {
                    Ok((_c, slot, _r, _l)) => format_slot(slot),
                    Err(e) => format!("<err: {e:#}>"),
                };

                let allow_slot = match find_allowance_slots_and_update_ratio(
                    &provider, holder, spender, token, None,
                ).await {
                    Ok((_c, slot, _r, _l)) => format_slot(slot),
                    Err(e) => format!("<err: {e:#}>"),
                };

                println!("{chain} | {symbol} | {address} | {bal_slot} | {allow_slot}");
            }
        }

        Ok(())
    }

}
