#!/bin/bash

BOOT_CONTRACTS_PATH="./tests/contracts/boot-contracts"
CLAR2WASM_PATH="../target/release"

declare -a boot_contracts=(
    bns
    cost-voting
    costs-2-testnet
    costs-2
    costs-3
    costs
    genesis
    lockup
    pox-mainnet-prepared
    pox-testnet-prepared
    pox-2-mainnet-prepared
    pox-2-testnet-prepared
    pox-3-mainnet-prepared
    pox-3-testnet-prepared
    pox-4
    signers
)

cat "${BOOT_CONTRACTS_PATH}/pox-mainnet.clar" "${BOOT_CONTRACTS_PATH}/pox.clar" >> "${BOOT_CONTRACTS_PATH}/pox-mainnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-testnet.clar" "${BOOT_CONTRACTS_PATH}/pox.clar" >> "${BOOT_CONTRACTS_PATH}/pox-testnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-mainnet.clar" "${BOOT_CONTRACTS_PATH}/pox-2.clar" >> "${BOOT_CONTRACTS_PATH}/pox-2-mainnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-testnet.clar" "${BOOT_CONTRACTS_PATH}/pox-2.clar" >> "${BOOT_CONTRACTS_PATH}/pox-2-testnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-mainnet.clar" "${BOOT_CONTRACTS_PATH}/pox-3.clar" >> "${BOOT_CONTRACTS_PATH}/pox-3-mainnet-prepared.clar"
cat "${BOOT_CONTRACTS_PATH}/pox-testnet.clar" "${BOOT_CONTRACTS_PATH}/pox-3.clar" >> "${BOOT_CONTRACTS_PATH}/pox-3-testnet-prepared.clar"

for contract in ${boot_contracts[@]}; do
    echo "Compiling ${contract}.clar file"
    "${CLAR2WASM_PATH}/clar2wasm" "${BOOT_CONTRACTS_PATH}/${contract}.clar"

    if [ $? == 0 ]; then
        echo "Compilation success"
    else
        echo "Failure while compiling ${contract}.clar"
        exit 1
    fi

    echo "Validating wasm binary ${contract}.wasm"
    wasm-tools validate "${BOOT_CONTRACTS_PATH}/${contract}.wasm"

    if [ $? == 0 ]; then
        echo "Binary validation success"
    else
        echo "Failure while validating wasm binary ${contract}.wasm"
        exit 1
    fi
done
