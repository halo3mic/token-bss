# TBSS Searcher 🦌

Token Balance Storage Slot Searcher

## Use case
Finding the storage slot of an ERC20 token can be useful for updating the balance of a token on a forked network.

#### Supported tokens
It supports the majority of ERC20 tokens accross Vyper and Solidity. Even proxies and cases for which storage contract where balances are stored is different than the token itself (eg. SNX). Note that for some contracts balance is not solely determined by the storage slot, so in those cases setting the storage slot to a specific value may not be exectly reflected in the balance - it could be slightly higher or lower.

## Components

### API

The API is live on: `http://token-bss.xyz`.

#### Supported Networks Keys
* ethereum / eth
* arbitrum / arb
* optimism / opt
* avalanche / avax


#### Example
Request:
```bash
$ curl http://token-bss.xyz/opt/0x513c7e3a9c69ca3e22550ef58ac1c0088e918fff | jq
```
Response:
```json
{
  "success": true,
  "msg": {
    "token": "0x513c7e3a9c69ca3e22550ef58ac1c0088e918fff",
    "contract": "0x513c7e3a9c69ca3e22550ef58ac1c0088e918fff",
    "slot": "0x34",
    "updateRatio": 1.0011,
    "lang": "solidity"
  }
}
```
-----
Checkout [Server README](./crates/server/README.md) for more information on usage.

### CLI Tool

#### Features 
- **Storage Slot Discovery**: Trace ERC20 `balanceOf` calls to determine the storage slot of a token and where is it stored.
- **Balance Updating**: Directly update the balance of an ERC20 token on a forked network.

![image](./assets/intro.gif)

Checkout [CLI README](./crates/cli/README.md) for more information on usage.


### Library
```rust
match token_bss::find_slot(&provider, token, None, None).await {
    Ok((contract, slot, update_ratio, lang)) => {
        println!("{symbol}({token:?}): {contract:?}({lang}) - {slot:?} / ΔR: {update_ratio}")
    }
    Err(e) => println!("{symbol}({token:?}): {e}"),
}
```

#### Run an Example
```bash
$ cargo run --example eth_token_support
```


----
    
Contributions are more than welcomed! 

Reach out on [X](https://twitter.com/MihaLotric)
