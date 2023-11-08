# Token Slot Finder

Token Slot Finder is a Rust-based command-line tool that aids with the discovery of storage slots and contracts for ERC20 tokens on EVM networks. It traces the `balanceOf` function call on the token contract to find the storage slot and the associated contract address.

## Features

- **Storage Slot Discovery**: Trace ERC20 `balanceOf` calls to determine the storage slot of a token and where is it stored.
- **Balance Updating**: Directly update the balance of an ERC20 token on a forked network.

## Installation

Clone the repository and build the tool using Cargo:

```bash
git clone https://github.com/halo3mic/erc20-topup.git
cd token-slot-finder
cargo build --release
```
## Usage
### Finding a Storage Slot


To find the storage slot of an ERC20 token, you can specify the RPC URL of an Anvil fork or a live network. Add the ``--unformatted` flag if you prefer the output in CSV format.

```bash
cargo run find-storage-slot [OPTIONS] --token-addresses <TOKEN_ADDRESSES>
```
Options
* `--rpc-url <RPC_URL>`: Specify the RPC URL of the Anvil fork.
* `--fork-rpc-url <FORK_RPC_URL>`: Specify the RPC URL of the live network.
* `--unformatted`: Output the result in an unformatted single line, separated by commas.
#### Example

```
$ cargo run -- find-storage-slot --fork-rpc-url $ETH_RPC 0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F,0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
```

```
Token: 0xc011a73ee8576fb46f5e1c5751ca3b9fe0af2a6f
Contract: 0x5b1b5fea1b99d83ad479df0c222f0492385381dd
Slot: 0x0000000000000000000000000000000000000000000000000000000000000003
Update ratio: 1
Language: solidity
```

### Updating a Token's Balance
Update the balance of an ERC20 token on an Anvil fork with the following command:

```
cargo run -- set-balance --rpc-url <ANVIL_FORK_RPC> <TOKEN> <HOLDER> <NEW_BALANCE>
```
### Output Slots from a Token List
Process a list of tokens and output their storage slots using the provided script.

```
./scripts/tokenlist_slots.sh <TOKEN_LIST_URL> [OPTIONS]
```
Options
* `-f/--fork_rpc_url` <LIVE_RPC>: Specify the RPC URL of the live network.
* `-n/--num_addresses` <LIMIT>: Limit the number of addresses processed.
* `-o/--output_file` <OUTPUT_FILE>: Specify the output file for the results.
#### Example
```
./scripts/tokenlist_slots.sh https://tokens.1inch.eth.link -f $LIVE_RPC -n 100 -o slots_output.txt
```

----
    
Contributions are more than welcomed! 

Reach out on [X](https://twitter.com/MihaLotric)
