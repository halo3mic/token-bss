#!/usr/bin/env bash

set -e

echo "Installing erc20-topup..."

BASE_DIR=${HOME}
ERC20_TOPUP_BIN_DIR="${BASE_DIR}/.erc20-topup/bin"
EXECUTABLE_PATH="${ERC20_TOPUP_BIN_DIR}/erc20-topup"

cargo build --release
if [ $? -ne 0 ]; then
    echo "Cargo build failed, exiting."
    exit 1
fi

mkdir -p "${ERC20_TOPUP_BIN_DIR}"
cp "./target/release/erc20-topup" "${EXECUTABLE_PATH}"
chmod +x "${EXECUTABLE_PATH}"

case $SHELL in
    */zsh)
        PROFILE="${HOME}/.zshrc"
        ;;
    */bash)
        PROFILE="${HOME}/.bashrc"
        ;;
    *)
        echo "Could not detect shell, manually add ${ERC20_TOPUP_BIN_DIR} to your PATH."
        exit 1
esac

if [[ ":$PATH:" != *":${ERC20_TOPUP_BIN_DIR}:"* ]]; then
    echo >> "${PROFILE}" && echo "export PATH=\"\$PATH:${ERC20_TOPUP_BIN_DIR}\"" >> "${PROFILE}"
fi

echo && echo "Added erc20-topup to PATH in ${PROFILE}. If the script was not sourced, run 'source ${PROFILE}' or start a new terminal session to use erc20-topup."
