# Gem — BelizeChain Smart Contract Platform

## Project Identity
- **Repo**: `BelizeChain/gem`
- **Role**: Smart contract platform — ink! 4.0 contracts for BelizeChain
- **Language**: Rust (ink! / Substrate)
- **Branch**: `main` (default)

## Contracts
- `dalla_token/` — PSP22 fungible token (DALLA)
- `beli_nft/` — PSP34 NFT standard
- `psp37_multi_token/` — PSP37 multi-token standard
- `access_control/` — Role-based access control
- `dex/` — Decentralized exchange
- `faucet/` — Token faucet for testnet
- `hello-belizechain/` — Example/starter contract

## Azure Deployment Target
- **ACR**: `belizechainacr.azurecr.io` → image: `belizechainacr.azurecr.io/gem`
- **AKS**: `belizechain-aks` (Free tier, 1x Standard_D2s_v3, K8s v1.33.6)
- **Resource Group**: `BelizeChain` in `centralus`
- **Subscription**: `77e6d0a2-78d2-4568-9f5a-34bd62357c40`
- **Tenant**: `belizechain.org`

## Deployment Status: Phase 2 — TODO
### What needs to be done:
1. **Determine deployment model** — Gem contracts are compiled to WASM and deployed ON-CHAIN via the belizechain node's `pallet-contracts`. They may not need a separate K8s service unless there's an API/tooling server.
2. **If API/SDK server exists**:
   - Verify/create Dockerfile
   - Update deploy.yml for AKS (same pattern as other services)
   - Push image to `belizechainacr.azurecr.io/gem`
   - Deploy as K8s service
3. **If contracts only (no server)**:
   - Build contracts: `cargo contract build --release`
   - Deploy WASM blobs to belizechain node via extrinsic
   - No separate K8s service needed — save AKS resources
4. **Configure GitHub Secrets** (if deploying a service):
   - `ACR_USERNAME` = `belizechainacr`
   - `ACR_PASSWORD` = (get from `az acr credential show --name belizechainacr`)
   - `AZURE_CREDENTIALS` = (service principal JSON)
   - `AZURE_RESOURCE_GROUP` = `BelizeChain`
   - `AKS_CLUSTER_NAME` = `belizechain-aks`
5. **Contract deployment**: Use `cargo-contract` or Polkadot.js to instantiate contracts on the running belizechain node

## Sibling Services (same AKS cluster)
| Service | Image | Ports |
|---------|-------|-------|
| belizechain-node | `belizechainacr.azurecr.io/belizechain-node` | 30333, 9944, 9615 |
| ui | `belizechainacr.azurecr.io/ui` | 80 |
| kinich-quantum | `belizechainacr.azurecr.io/kinich` | 8000 |
| nawal-ai | `belizechainacr.azurecr.io/nawal` | 8001 |
| pakit-storage | `belizechainacr.azurecr.io/pakit` | 8002 |

## Dev Commands
```bash
cargo contract build --release           # Build contract WASM
cargo test                               # Run contract tests
cargo contract upload --url ws://127.0.0.1:9944  # Deploy locally
```

## Constraints
- ink! contracts compile to WASM — executed inside the blockchain runtime, not standalone
- Contracts share on-chain resources with all other pallets
- AKS cost ceiling: ~$75/mo total for ALL services — prefer on-chain deployment over separate service
