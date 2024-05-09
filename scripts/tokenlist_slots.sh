#!/bin/bash

# Supports any url with tokenlist standard (https://github.com/Uniswap/token-lists).
#   Find yours on https://tokenlists.org/(eg. https://tokens.1inch.eth.link/) 
#   or using coingecko api: https://tokens.coingecko.com/{ uniswap | ethereum | arbitrum-one | ... }/all.json.


if [ -z "$1" ]; then
  echo "Error: Missing required URL argument"
  exit 1
fi
tokenlist_url="$1"
fork_rpc_url=${RPC_URL:-"http://localhost:8545"}


num_addresses=100
output_file="output.csv"
while [ "$#" -gt 1 ]; do
  case "$2" in
    -f|--fork-rpc-url)
      fork_rpc_url="$3"
      shift 2
      ;;
    -n|--num-addresses)
      num_addresses="$3"
      shift 2
      ;;
    -o|--output-file)
      output_file="$3"
      shift 2
      ;;
    *)
      echo "Unknown argument: $2"
      exit 1
      ;;
  esac
done

a=$(curl $tokenlist_url | jq -r '.tokens[] | .address' | head -n "$num_addresses" |  tr '\n' ',')
echo $((${#a} / 43)) "tokens found"
echo "Token,Contract,Slot,UpdateRatio,Language,Error" > "$output_file"
./target/release/token-bss find-storage-slot --unformatted --fork-rpc-url "$fork_rpc_url" "$a" >> "$output_file"
echo "Completed! ✨"
