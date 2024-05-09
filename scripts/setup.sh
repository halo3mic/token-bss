#!/usr/bin/env bash

set -e

echo "Installing token-bss..."

BASE_DIR=${HOME}
TOKEN_BSS_BIN_DIR="${BASE_DIR}/.token-bss/bin"
EXECUTABLE_PATH="${TOKEN_BSS_BIN_DIR}/token-bss"

cargo build --bin token-bss-cli --release
if [ $? -ne 0 ]; then
    echo "Cargo build failed, exiting."
    exit 1
fi

mkdir -p "${TOKEN_BSS_BIN_DIR}"
cp "./target/release/token-bss-cli" "${EXECUTABLE_PATH}"
chmod +x "${EXECUTABLE_PATH}"

case $SHELL in
    */zsh)
        PROFILE="${HOME}/.zshrc"
        ;;
    */bash)
        PROFILE="${HOME}/.bashrc"
        ;;
    *)
        echo "Could not detect shell, manually add ${TOKEN_BSS_BIN_DIR} to your PATH."
        exit 1
esac

if [[ ":$PATH:" != *":${TOKEN_BSS_BIN_DIR}:"* ]]; then
    echo >> "${PROFILE}" && echo "export PATH=\"\$PATH:${TOKEN_BSS_BIN_DIR}\"" >> "${PROFILE}"
fi

echo && echo "Added token-bss to PATH in ${PROFILE}. If the script was not sourced, run 'source ${PROFILE}' or start a new terminal session to use token-bss."
