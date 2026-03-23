#!/bin/bash
# Deploy GEM smart contracts to BelizeChain testnet
#
# Prerequisites:
#   1. Contracts built: bash scripts/build-all.sh
#   2. Node accessible at WS endpoint
#   3. Node.js deps installed: npm install (in project root)
#
# Usage:
#   bash scripts/deploy-testnet.sh                  # Uses //Alice account
#   DEPLOY_ACCOUNT="//Bob" bash scripts/deploy-testnet.sh
#
# For in-cluster (AKS) deployment, the WS URL defaults to:
#   ws://belizechain-node.belizechain.svc.cluster.local:9944
#
# Override with:
#   BLOCKCHAIN_WS_URL="ws://custom-node:9944" bash scripts/deploy-testnet.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "╔════════════════════════════════════════════════════════╗"
echo "║   BelizeChain GEM — Testnet Contract Deployment       ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# -------------------------------------------------------------------
# 1. Pre-flight checks
# -------------------------------------------------------------------

# Check that contract artifacts exist
MISSING=0
for artifact in \
    dalla_token/target/ink/dalla_token.contract \
    beli_nft/target/ink/beli_nft.contract \
    simple_dao/target/ink/simple_dao.contract \
    faucet/target/ink/faucet.contract; do
    if [ ! -f "$PROJECT_DIR/$artifact" ]; then
        echo "❌ Missing artifact: $artifact"
        MISSING=1
    fi
done

if [ "$MISSING" -eq 1 ]; then
    echo ""
    echo "Run 'bash scripts/build-all.sh' first to build contracts."
    exit 1
fi
echo "✅ Contract artifacts found"

# Check node_modules (SDK contains the @polkadot dependencies)
SDK_DIR="$PROJECT_DIR/sdk"
if [ ! -d "$SDK_DIR/node_modules" ]; then
    echo "📦 Installing Node.js dependencies..."
    (cd "$SDK_DIR" && npm install --no-audit --no-fund)
fi
export NODE_PATH="$SDK_DIR/node_modules"
echo "✅ Node.js dependencies ready"
echo ""

# -------------------------------------------------------------------
# 2. Set testnet environment
# -------------------------------------------------------------------

export DEPLOY_ACCOUNT="${DEPLOY_ACCOUNT:-//Alice}"

# Default to AKS internal DNS; override with BLOCKCHAIN_WS_URL
if [ -z "${BLOCKCHAIN_WS_URL:-}" ]; then
    # Try local node first (for dev machines), fall back to AKS internal DNS
    if command -v nc &>/dev/null && nc -z localhost 9944 2>/dev/null; then
        export BLOCKCHAIN_WS_URL="ws://localhost:9944"
        echo "🔗 Using local node: $BLOCKCHAIN_WS_URL"
    else
        export BLOCKCHAIN_WS_URL="ws://belizechain-node.belizechain.svc.cluster.local:9944"
        echo "🔗 Using AKS testnet node: $BLOCKCHAIN_WS_URL"
    fi
else
    echo "🔗 Using custom endpoint: $BLOCKCHAIN_WS_URL"
fi

echo "👤 Deploy account: $DEPLOY_ACCOUNT"
echo ""

# -------------------------------------------------------------------
# 3. Deploy contracts
# -------------------------------------------------------------------

echo "🚀 Deploying all contracts to testnet..."
echo ""

cd "$PROJECT_DIR"
node scripts/deploy.js --contract=all --network=testnet --account="$DEPLOY_ACCOUNT"

echo ""
echo "════════════════════════════════════════════════════════"
echo "✅ Testnet deployment complete!"
echo ""
echo "Deployed contracts:"
echo "  • DALLA Token (PSP22) — 1M DALLA initial supply"
echo "  • BeliNFT (PSP34)"
echo "  • Simple DAO"
echo "  • Faucet — 100 DALLA per claim, ~10 min cooldown"
echo ""
echo "Deployment record saved to: deployments/"
echo "════════════════════════════════════════════════════════"
