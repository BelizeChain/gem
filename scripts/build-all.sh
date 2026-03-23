#!/bin/bash
# Build all contracts before deployment
# This ensures all .contract files are up-to-date

set -e  # Exit on any error

echo "╔════════════════════════════════════════════════════════╗"
echo "║   Building All GEM Contracts for Deployment           ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Check if cargo-contract is installed
if ! command -v cargo-contract &> /dev/null; then
    echo "❌ cargo-contract is not installed"
    echo ""
    echo "Install it with:"
    echo "  cargo install cargo-contract --force --locked"
    exit 1
fi

echo "✅ cargo-contract found: $(cargo-contract --version)"
echo ""

# Function to build a contract
build_contract() {
    local name=$1
    local path=$2

    echo "📦 Building $name..."
    if cargo contract build --release --manifest-path "$path/Cargo.toml" --quiet; then
        echo "   ✅ $name built successfully"

        # Check if .contract file was created
        local contract_file="${path}/target/ink/*.contract"
        if ls $contract_file 1> /dev/null 2>&1; then
            local size=$(du -h $contract_file | cut -f1)
            echo "      Contract file: $size"
        fi
    else
        echo "   ❌ Failed to build $name"
        return 1
    fi
    echo ""
}

# Build order (standalone contracts first)
# Note: access_control is a library, not a deployable contract — verified via cargo check
declare -a CONTRACTS=(
    "DALLA Token:dalla_token"
    "BeliNFT Collection:beli_nft"
    "Simple DAO:simple_dao"
    "Faucet:faucet"
    "PSP37 Multi-Token:psp37_multi_token"
    "BelizeX Factory:dex/factory"
    "BelizeX Pair:dex/pair"
    "BelizeX Router:dex/router"
)

# Track success/failure
SUCCESS_COUNT=0
TOTAL_COUNT=${#CONTRACTS[@]}

# Build each contract
for contract_info in "${CONTRACTS[@]}"; do
    IFS=':' read -r name path <<< "$contract_info"
    if build_contract "$name" "$path"; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
done

echo "════════════════════════════════════════════════════════"
echo "📊 Build Summary: $SUCCESS_COUNT/$TOTAL_COUNT contracts built successfully"

if [ $SUCCESS_COUNT -eq $TOTAL_COUNT ]; then
    echo "✅ All contracts ready for deployment!"
    echo ""
    echo "Next steps:"
    echo "  1. Check if node is running: node scripts/check-node.js"
    echo "  2. Deploy contracts: node scripts/deploy.js"
    exit 0
else
    echo "❌ Some contracts failed to build"
    exit 1
fi
