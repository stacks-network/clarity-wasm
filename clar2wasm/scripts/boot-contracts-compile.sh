#!/bin/bash

BOOT_CONTRACTS_PATH="./tests/contracts/boot-contracts"
CLAR2WASM_PATH="../target/debug/clar2wasm"

declare -a boot_contracts=(
    bns.clar
    cost-voting.clar
    costs-2-testnet.clar
    costs-2.clar
    costs-3.clar
    costs.clar
    genesis.clar
    lockup.clar
    pox-mainnet-prepared.clar
    pox-testnet-prepared.clar
    pox-2-mainnet-prepared.clar
    pox-2-testnet-prepared.clar
    pox-3-mainnet-prepared.clar
    pox-3-testnet-prepared.clar
    pox-4.clar
    signers.clar
)

cat "${BOOT_CONTRACTS_PATH}/pox-mainnet.clar" "${BOOT_CONTRACTS_PATH}/pox.clar" >> "${BOOT_CONTRACTS_PATH}/pox-mainnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-testnet.clar" "${BOOT_CONTRACTS_PATH}/pox.clar" >> "${BOOT_CONTRACTS_PATH}/pox-testnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-mainnet.clar" "${BOOT_CONTRACTS_PATH}/pox-2.clar" >> "${BOOT_CONTRACTS_PATH}/pox-2-mainnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-testnet.clar" "${BOOT_CONTRACTS_PATH}/pox-2.clar" >> "${BOOT_CONTRACTS_PATH}/pox-2-testnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-mainnet.clar" "${BOOT_CONTRACTS_PATH}/pox-3.clar" >> "${BOOT_CONTRACTS_PATH}/pox-3-mainnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-testnet.clar" "${BOOT_CONTRACTS_PATH}/pox-3.clar" >> "${BOOT_CONTRACTS_PATH}/pox-3-testnet-prepared.clar"

for contract in ${boot_contracts[@]}; do
    echo "Compiling $contract file..."
    "${CLAR2WASM_PATH}" "${BOOT_CONTRACTS_PATH}/$contract"

    if [ $? == 0 ]; then
        echo "Success"
    else
        echo "Failure while compiling $contract"
        exit 1
    fi
done
