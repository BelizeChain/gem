# ──────────────────────────────────────────────────────────
# GEM Contract Deployer
# Builds all ink! contracts and packages deploy tooling
# ──────────────────────────────────────────────────────────

# Stage 1: Build contracts
FROM rust:1.90-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    binaryen \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-contract --version 4.1.1 --locked

WORKDIR /build

# Copy workspace-level files
COPY Cargo.toml rust-toolchain.toml ./

# Copy all contracts
COPY dalla_token/ dalla_token/
COPY beli_nft/ beli_nft/
COPY simple_dao/ simple_dao/
COPY faucet/ faucet/
COPY access_control/ access_control/
COPY psp37_multi_token/ psp37_multi_token/
COPY dex/ dex/
COPY hello-belizechain/ hello-belizechain/

# Build all contracts in release mode
RUN for contract in \
      dalla_token \
      beli_nft \
      simple_dao \
      faucet \
      access_control \
      psp37_multi_token \
      dex/factory \
      dex/pair \
      dex/router; do \
    echo "Building $contract..." && \
    cargo contract build --release --manifest-path "$contract/Cargo.toml" || exit 1; \
  done

# Stage 2: Deploy runtime (Node.js + contract artifacts)
FROM node:20-slim

WORKDIR /app

# Copy SDK and deploy scripts
COPY sdk/ sdk/
COPY scripts/ scripts/

# Install SDK dependencies
WORKDIR /app/sdk
RUN npm ci --omit=dev 2>/dev/null || npm install --omit=dev

WORKDIR /app

# Copy built contract artifacts from builder
COPY --from=builder /build/dalla_token/target/ink/ artifacts/dalla_token/
COPY --from=builder /build/beli_nft/target/ink/ artifacts/beli_nft/
COPY --from=builder /build/simple_dao/target/ink/ artifacts/simple_dao/
COPY --from=builder /build/faucet/target/ink/ artifacts/faucet/
COPY --from=builder /build/access_control/target/ink/ artifacts/access_control/
COPY --from=builder /build/psp37_multi_token/target/ink/ artifacts/psp37_multi_token/
COPY --from=builder /build/dex/factory/target/ink/ artifacts/dex_factory/
COPY --from=builder /build/dex/pair/target/ink/ artifacts/dex_pair/
COPY --from=builder /build/dex/router/target/ink/ artifacts/dex_router/

# Default: run the deployment script
CMD ["node", "scripts/deploy.js"]
