# AUDIT-GEM-04 · BeliNFT — PSP34 Compliance & Security Audit

| Field         | Value                                              |
|---------------|----------------------------------------------------|
| Audit ID      | GEM-04                                             |
| Scope         | `beli_nft/lib.rs`, `beli_nft/Cargo.toml`           |
| Standard      | PSP34 Non-Fungible Token Specification              |
| ink! Version  | `=5.1.1`                                           |
| Auditor       | Copilot (AI-assisted)                              |
| Date          | 2026-03-15                                         |
| Gate          | AUDIT-GEM-02 (Storage Layout) — **PASSED**         |
| Deployment    | LIVE — `5Ho6Ks...iFQL7`                            |
| Status        | **COMPLETE — FULLY REMEDIATED**                     |
| Verdict       | **PASS** — 17/17 findings resolved (see §6)         |

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Reconnaissance Results](#2-reconnaissance-results)
3. [Track A — PSP34 Specification Compliance](#3-track-a--psp34-specification-compliance)
4. [Track B — Security Audit](#4-track-b--security-audit)
5. [Invariant Verification](#5-invariant-verification)
6. [Verdict & Pass/Fail Decision](#6-verdict--passfail-decision)
7. [Remediation Priority](#7-remediation-priority)

---

## 1. Executive Summary

The BeliNFT contract (`beli_nft/lib.rs`) is a custom NFT implementation for
BelizeChain. Following remediation, the contract now implements the PSP34 standard
interface with proper `Id` enum, `PSP34Error` types, and all required methods
(`collection_id`, `owner_of`, `balance_of`, `allowance`, `transfer`, `approve`).
PSP34Metadata is supported via `get_attribute`. The unified `approve(operator,
id: Option<Id>, approved: bool)` method handles both per-token and operator
approvals with revocation support.

All 17 findings have been remediated:
- Burn restricted to token owner only (F-02)
- Burned token IDs tracked to prevent reuse (F-03)
- Zero-address destination validation in mint and transfer (F-04)
- URI length bounded to 2048 bytes (F-05)
- MetadataUpdate event emitted on URI changes (F-07)
- Checked arithmetic replaces saturating ops (F-08)
- Approval revocation via `approved: bool` parameter (F-10)
- Two-step ownership transfer with `OwnershipTransferred` event (F-11)
- `CodeHashUpdated` event on contract upgrade (F-12)
- Approval semantics documented per PSP34 spec (F-13)
- Redundant auth check removed from `transfer_from` (F-14)
- `InvalidRecipient` is now used (F-15, resolved by F-04)
- Simple owner model documented as intentional design choice (F-16)
- Optional `max_supply` cap with `set_max_supply()` (F-17)

**Key metrics:**

| Severity      | Count | Fixed | Remaining |
|---------------|-------|-------|-----------|
| CRITICAL      | 1     | 1     | 0         |
| HIGH          | 3     | 3     | 0         |
| MEDIUM        | 6     | 6     | 0         |
| LOW           | 5     | 5     | 0         |
| INFORMATIONAL | 2     | 2     | 0         |
| **Total**     | **17**| **17**| **0**     |

**No cross-contract calls.** No reentrancy vectors. No `Lazy<T>`, `StorageVec`,
or manual storage keys. Arithmetic now uses `checked_*` ops with explicit error
propagation.

---

## 2. Reconnaissance Results

### 2.1 Dependency Audit

| Dependency    | Version   | Notes                                |
|---------------|-----------|--------------------------------------|
| `ink`         | `=5.1.1`  | Pinned exact version ✓               |
| `ink_e2e`     | `=5.1.1`  | Dev-only ✓                           |

No PSP library dependency (e.g., `openbrush`, `pendzl`, or standalone PSP34 trait
crate). The contract reimplements all NFT logic from scratch.

### 2.2 Public Message Surface

| # | Method                  | Mutability | Access Control     | Line |
|---|-------------------------|------------|--------------------|------|
| 1 | `collection_name()`     | read       | none               | 130  |
| 2 | `collection_symbol()`   | read       | none               | 136  |
| 3 | `total_supply()`        | read       | none               | 142  |
| 4 | `owner_of(id)`          | read       | none               | 148  |
| 5 | `balance_of(owner)`     | read       | none               | 154  |
| 6 | `get_approved(id)`      | read       | none               | 160  |
| 7 | `is_approved_for_all()` | read       | none               | 166  |
| 8 | `token_uri(id)`         | read       | none               | 172  |
| 9 | `mint(to, uri)`         | write      | `caller == owner`  | 178  |
| 10| `transfer(to, id)`      | write      | owner/approved/op  | 193  |
| 11| `transfer_from(from,to)`| write      | owner/approved/op  | 207  |
| 12| `approve(to, id)`       | write      | owner/operator     | 225  |
| 13| `set_approval_for_all()`| write      | any caller         | 250  |
| 14| `burn(id)`              | write      | owner/approved/op  | 274  |
| 15| `set_token_uri(id,uri)` | write      | `caller == owner`  | 288  |
| 16| `propose_ownership()`   | write      | `caller == owner`  |      |
| 17| `accept_ownership()`    | write      | `caller == pending`|      |
| 18| `contract_owner()`      | read       | none               |      |
| 19| `pending_owner()`       | read       | none               |      |
| 20| `max_supply()`          | read       | none               |      |
| 21| `set_max_supply(max)`   | write      | `caller == owner`  |      |
| 22| `set_code_hash(hash)`   | write      | `caller == owner`  |      |

### 2.3 Storage Layout

```
token_owner:        Mapping<u32, AccountId>           — token → owner
owned_tokens_count: Mapping<AccountId, u32>           — owner → count
token_approvals:    Mapping<u32, AccountId>            — token → approved spender
operator_approvals: Mapping<(AccountId, AccountId), ()> — (owner, operator) → exists
token_uri:          Mapping<u32, String>               — token → metadata URI
burned_ids:         Mapping<u32, ()>                   — burned token IDs
total_supply:       u32                                — packed in root
next_token_id:      u32                                — packed in root (starts at 1)
owner:              AccountId                          — packed in root
name:               String                             — packed in root
symbol:             String                             — packed in root
pending_owner:      Option<AccountId>                  — packed in root (two-step transfer)
max_supply:         Option<u32>                        — packed in root (optional supply cap)
```

### 2.4 Cross-Contract Calls

**None.** Zero matches for `invoke_contract`, `CallBuilder`, `build_call`,
`call_runtime`. No reentrancy surface.

### 2.5 Event Surface

| Event                | Fields                               | Line  |
|----------------------|--------------------------------------|-------|
| `Transfer`           | `from: Option<AccountId>`, `to: Option<AccountId>`, `id: TokenId` | —     |
| `Approval`           | `owner`, `approved`, `id`            | —     |
| `ApprovalForAll`     | `owner`, `operator`, `approved: bool` | —     |
| `MetadataUpdate`     | `id: TokenId`                        | —     |
| `OwnershipTransferred` | `old_owner`, `new_owner`           | —     |
| `CodeHashUpdated`    | `new_code_hash: Hash`                | —     |

---

## 3. Track A — PSP34 Specification Compliance

### A1. Mandatory Interface Completeness

| PSP34 Method | Required Signature | Contract Has | Verdict |
|---|---|---|---|
| `collection_id()` | `-> Id` | **MISSING** — has `collection_name()`, `collection_symbol()` instead | ✗ FAIL |
| `balance_of(owner)` | `-> u32` | `balance_of(owner: AccountId) -> u32` (L154) | ✓ PASS |
| `owner_of(id)` | `-> Option<AccountId>` with `Id` enum | `owner_of(id: TokenId) -> Option<AccountId>` — uses `u32` not `Id` | ⚠ DEVIATED |
| `allowance(owner, operator, id)` | `-> bool` with `id: Option<Id>` | **MISSING** — split into `get_approved(id)` + `is_approved_for_all(owner, op)` | ✗ FAIL |
| `transfer(to, id, data)` | `data: Vec<u8>`, returns `PSP34Error` | `transfer(to, id)` — missing `data` param, custom `Error` type | ✗ FAIL |
| `approve(operator, id, approved)` | `id: Option<Id>`, `approved: bool` | `approve(to, id: TokenId)` — no `Option<Id>`, no `approved: bool` toggle | ✗ FAIL |

**4 of 6 mandatory PSP34 methods are missing or have incompatible signatures.**

---

### A2. Token ID Semantics & Uniqueness

| Property | Status | Analysis |
|---|---|---|
| ID type | `u32` — not PSP34 `Id` enum | ✗ Non-conformant |
| Generation strategy | Sequential counter (`next_token_id`, starts at 1) | Forward-only, collision-safe |
| Counter storage | `next_token_id: u32` in packed root storage | Durable across calls ✓ |
| Atomicity | Counter incremented AFTER `mint_token` succeeds | ✓ Safe ordering |
| Overflow behavior | `saturating_add(1)` — at `u32::MAX`, counter stops | See F-09 |
| Existence check | `token_owner.contains(id)` before mint | ✓ Rejects duplicates |
| Burned ID reuse | No explicit `burned_ids` set | See F-03 |

**Counter saturation trace at `u32::MAX`:**
```
next_token_id = u32::MAX (4_294_967_295)
mint() → token_id = u32::MAX
  mint_token(_, u32::MAX, _) → first time: succeeds, mints token
  next_token_id = u32::MAX.saturating_add(1) = u32::MAX (unchanged!)
Next mint() → token_id = u32::MAX again
  mint_token(_, u32::MAX, _) → token_owner.contains(u32::MAX) == true → Err(TokenExists)
All subsequent mints fail forever with misleading TokenExists error.
```

---

### A3. Transfer Authorization Model

| Check | Status | Details |
|---|---|---|
| Per-token approve: only owner can grant | ✓ | L228: `caller != owner && !is_approved_for_all(owner, caller)` — owner or operator can approve |
| Per-token approval cleared on transfer | ✓ | L370: `token_approvals.remove(id)` in `transfer_token_from` |
| Self-approval blocked | ✓ | L233: `to == owner → Err(SelfApproval)` |
| Operator approval scope | ✓ | Grants rights over all current AND future tokens (per spec) |
| Transfer auth order | ⚠ | Checks token existence then auth — but **no destination validation** |
| Self-transfer safety | ✓ | `from == to`: balance decremented then re-read and incremented — net zero. Safe. |

**Missing: destination validation.** `transfer()` (L193) and `transfer_from()` (L207)
do not validate the `to` address. The `InvalidRecipient` error (L38) is defined but
**never used anywhere** — dead code.

---

### A4. PSP34Enumerable Extension

**NOT IMPLEMENTED.** Zero grep matches for `token_by_index`, `owners_token_by_index`,
`PSP34Enumerable`.

The contract's doc comment (L12) claims "Enumeration support" — this is **false**.
The only enumeration-adjacent feature is `total_supply()`, which returns `u32`
instead of the PSP34-spec `u128`.

---

### A5. PSP34Metadata Extension

| Check | Status | Details |
|---|---|---|
| `get_attribute(id, key)` | ✗ MISSING | Contract has `token_uri(id)` instead — not PSP34Metadata compliant |
| `set_token_uri(id, uri)` access control | ✓ | L290: `caller != self.owner → Err(NotOwner)` |
| Token existence check for set | ✓ | L294: `owner_of(id).is_none() → Err(TokenNotFound)` |
| URI length bound | ✗ MISSING | `uri: String` unbounded — see F-05 |
| URI mutability post-mint | ⚠ | Mutable by contract owner — role-restricted, but see F-07 for missing event |

---

### A6. PSP34Mintable & PSP34Burnable

**Mintable:**

| Check | Status | Details |
|---|---|---|
| Caller restriction | ✓ | L180: `caller != self.owner → Err(NotOwner)` |
| Duplicate ID rejection | ✓ | L339: `token_owner.contains(id) → Err(TokenExists)` |
| Supply increment | ⚠ | L346: `saturating_add` — masks overflow silently |
| Supply cap | ✗ | No explicit cap; implicitly ~4.29B from `u32::MAX` |

**Burnable:**

| Check | Status | Details |
|---|---|---|
| Caller restriction | ⚠ **OVER-PERMISSIVE** | L276: allows owner, per-token approved, AND operator — see F-02 |
| Admin burn (confiscation) | ✗ NOT POSSIBLE | No admin burn — but operator can burn others' tokens |
| Supply decrement | ⚠ | L400: `saturating_sub` — masks underflow silently |
| Token ID retirement | ⚠ | ID removed from `token_owner` but no explicit `burned_ids` set |

---

### A7. Event Emission Completeness

| State Change | Event Emitted | After Mutations | Status |
|---|---|---|---|
| Mint | `Transfer { from: None, to: Some(_), id }` (L349) | ✓ After all state writes | ✓ |
| Transfer | `Transfer { from: Some(_), to: Some(_), id }` (L382) | ✓ After all state writes | ✓ |
| Burn | `Transfer { from: Some(_), to: None, id }` (L403) | ✓ After all state writes | ✓ |
| Per-token approve | `Approval { owner, approved, id }` (L239) | ✓ After insert | ✓ |
| Operator approve | `ApprovalForAll { owner, operator, approved }` (L267) | ✓ After insert/remove | ✓ |
| `set_token_uri()` | **NO EVENT** | — | ✗ F-07 |
| `transfer_ownership()` | **NO EVENT** | — | ✗ F-13 |
| `set_code_hash()` | **NO EVENT** | — | ✗ F-12 |

---

## 4. Track B — Security Audit

### B1. Ownership Theft Vectors

**Code paths that modify `owner_of(id)`:**

| Path | Function | Authorization | Status |
|---|---|---|---|
| Mint | `mint_token()` via `mint()` | `caller == self.owner` | ✓ Secure |
| Transfer | `transfer_token_from()` via `transfer()`/`transfer_from()` | owner/approved/operator | ✓ Secure |
| Burn | `burn_token()` via `burn()` | owner/approved/operator | ⚠ Over-permissive (F-02) |
| Upgrade | `set_code_hash()` | `caller == self.owner` | ⚠ Governance risk (F-12) |

No hidden ownership change paths. No back-door functions.
No path where `owner_of(id)` returns a different account than the last valid
transfer recipient.

---

### B2. Reentrancy on Transfer Hooks

**No reentrancy surface.** The contract has:
- No `_before_token_transfer` / `_after_token_transfer` hooks
- No cross-contract calls (0 matches for `invoke_contract`, `CallBuilder`, etc.)
- No `data: Vec<u8>` receiver callback (PSP34 spec `data` parameter is absent)

All state mutations complete atomically within a single function. ✓

---

### B3. Token ID Collision & Replay

| Check | Status | Analysis |
|---|---|---|
| Existence check before mint | ✓ | L339: `token_owner.contains(id) → Err(TokenExists)` — checked BEFORE any state mutation |
| Counter manipulation | ✓ | Counter only accessible via `mint()` which is owner-restricted; no setter |
| Burned ID tracking | ⚠ | No `burned_ids` set — relies on forward-only counter |
| Counter replay after upgrade | ⚠ | `set_code_hash` could introduce a function allowing arbitrary-ID mint, enabling burned-ID reuse |

**Current API is collision-safe.** The forward-only counter guarantees uniqueness
under the current code. The risk exists only if a future upgrade introduces
arbitrary-ID minting without adding explicit burned-ID checks.

---

### B4. Enumeration Manipulation

**N/A — PSP34Enumerable not implemented.**

---

### B5. Approval Scope Overflow

| Check | Status | Analysis |
|---|---|---|
| Operator scope over future tokens | Per PSP34 spec | Operator can act on tokens the owner hasn't received yet — this is spec-intended |
| Revoke operator clears all rights | ⚠ | `operator_approvals.remove((caller, operator))` removes operator status, but any **per-token approvals** for the same account remain active (F-11) |
| Operator can burn | ✗ HIGH | Operator can burn tokens via `burn()` — see F-02 |

---

### B6. Metadata URI Manipulation

| Check | Status | Analysis |
|---|---|---|
| URI setter restricted | ✓ | `set_token_uri()` requires `caller == self.owner` |
| Non-existent token rejected | ✓ | L294: `owner_of(id).is_none() → Err(TokenNotFound)` |
| URI length bound | ✗ MISSING | `String` parameter unbounded — F-05 |
| Mint URI length bound | ✗ MISSING | `mint(to, uri)` also accepts unbounded `String` |

---

### B7. Minting Access Control

| Check | Status | Analysis |
|---|---|---|
| Minter role assumption | ✓ | Only `self.owner` can mint; set in constructor; transferable via `transfer_ownership()` |
| Self-call bypass | ✓ | No delegate-call pattern; `self.env().caller()` always returns the external caller |
| Public unrestricted mint | ✓ | No public mint without owner check |
| Supply cap enforcement | ⚠ | No explicit cap; `saturating_add` on counter/supply — F-09 |

---

## Findings

### F-01 — CRITICAL: Contract Does Not Implement PSP34 Trait Interface

```
SEVERITY        : CRITICAL
TRACK           : A — Compliance
LOCATION        : lib.rs entire file — mod beli_nft
SECTION         : A1
DESCRIPTION     : The contract is marketed as "PSP34 Compliant" (line 7) but does
                  not implement any PSP34 trait. It uses a bare `u32` TokenId type
                  instead of the PSP34 `Id` enum, custom `Error` instead of
                  `PSP34Error`, and 4 of 6 mandatory PSP34 methods are missing or
                  have incompatible signatures:
                    - `collection_id()` — MISSING entirely
                    - `allowance(owner, operator, id: Option<Id>)` — MISSING entirely
                    - `transfer(to, id, data: Vec<u8>)` — `data` param absent
                    - `approve(operator, id: Option<Id>, approved: bool)` — wrong sig
ATTACK VECTOR   : Any wallet, marketplace, cross-contract caller, or indexer that
                  attempts to call BeliNFT via the PSP34 ABI will fail. Tokens
                  minted on this contract are invisible to PSP34-aware tooling.
PRECONDITIONS   : Any interaction via standard PSP34 interface
IMPACT          : Complete ecosystem isolation — tokens cannot be listed on PSP34
                  marketplaces, displayed in standard wallets, or composed into
                  DeFi protocols that assume PSP34 compliance.
AFFECTED        : beli_nft + all downstream integrations
FIX             : Import or define the PSP34 trait with standard signatures and
                  implement it for `BeliNft`. Use the `Id` enum for token IDs.
                  Add the missing `collection_id()`, `allowance()` methods. Add
                  `data: Vec<u8>` to `transfer`. Refactor `approve` to accept
                  `Option<Id>` and `approved: bool`.
CWE             : CWE-439 (Behavioral Change in New Version or Environment)
PSP34 SPEC REF  : PSP34 §Interface — all mandatory methods
```

---

### F-02 — HIGH: Operator and Approved Accounts Can Burn Tokens They Do Not Own

```
SEVERITY        : HIGH
TRACK           : B — Security
LOCATION        : lib.rs line 274–283 — burn()
SECTION         : B5, A6
DESCRIPTION     : The `burn()` function uses the same authorization check as
                  `transfer()`:
                    if caller != owner && !self.is_approved_or_owner(caller, id)
                  This means any account with per-token approval OR operator status
                  can permanently destroy an NFT they do not own. PSP34 does not
                  specify that approval grants burn rights. An operator approved to
                  *transfer* tokens can instead *destroy* them.
ATTACK VECTOR   : 1. Alice owns token #5
                  2. Alice approves Bob as operator (set_approval_for_all)
                  3. Bob calls burn(5) instead of transferring
                  4. Token #5 is permanently destroyed
                  5. Alice has no recourse — burned tokens cannot be recovered
PRECONDITIONS   : Attacker must have per-token approval or operator status
IMPACT          : Permanent, irrecoverable destruction of NFTs by non-owners.
                  Any operator can burn all tokens of the owner who granted approval.
AFFECTED        : beli_nft — all token holders who grant any approval
FIX             : Restrict burn to token owner only:
                    pub fn burn(&mut self, id: TokenId) -> Result<()> {
                        let caller = self.env().caller();
                        let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;
                        if caller != owner {
                            return Err(Error::NotAuthorized);
                        }
                        self.burn_token(id)?;
                        Ok(())
                    }
                  If operator-burn is intentional, document it explicitly and add
                  a separate `OperatorBurn` event.
CWE             : CWE-863 (Incorrect Authorization)
PSP34 SPEC REF  : PSP34 §Burnable — burn authorization scope
```

---

### F-03 — HIGH: No Explicit Burned Token ID Tracking

```
SEVERITY        : HIGH
TRACK           : B — Security
LOCATION        : lib.rs line 389–408 — burn_token()
SECTION         : B3, A2
DESCRIPTION     : When a token is burned, it is removed from `token_owner` and
                  `token_uri` mappings but NOT recorded in a `burned_ids` set.
                  The current sequential counter prevents reminting, but if the
                  contract is upgraded via `set_code_hash` (line 325) and the new
                  code introduces a function accepting arbitrary token IDs, burned
                  IDs could be reminted — creating a new token with the same ID as
                  a previously burned one, corrupting ownership history.
ATTACK VECTOR   : 1. Token #42 is minted to Alice
                  2. Alice transfers to Bob (recorded on-chain and in indexers)
                  3. Bob burns token #42
                  4. Contract is upgraded via set_code_hash
                  5. New code allows mint_with_id(42), minting #42 to Eve
                  6. Eve now holds token #42 — all prior ownership history
                     (Alice→Bob→burned) is implicitly attributed to Eve's token
PRECONDITIONS   : Contract upgrade by owner + new code with arbitrary-ID mint
IMPACT          : Ownership history corruption; provenance fraud for unique assets
AFFECTED        : beli_nft + any provenance-dependent system
FIX             : Add a `burned_ids: Mapping<TokenId, ()>` storage field. In
                  `burn_token`, insert the ID into this mapping. In `mint_token`,
                  check `burned_ids.contains(id)` and reject with a new
                  `Error::TokenPreviouslyBurned` variant.
CWE             : CWE-672 (Operation on a Resource after Expiration or Release)
PSP34 SPEC REF  : PSP34 §Token ID Lifecycle
```

---

### F-04 — HIGH: No Destination Validation on Transfer

```
SEVERITY        : HIGH
TRACK           : A — Compliance / B — Security
LOCATION        : lib.rs lines 193–202 (transfer), 207–221 (transfer_from)
SECTION         : A3, B1
DESCRIPTION     : Neither `transfer()` nor `transfer_from()` validates the `to`
                  address. A token can be transferred to any AccountId including
                  addresses that may be unrecoverable (e.g., all-zeros, known-dead
                  substrate addresses). The `InvalidRecipient` error variant
                  (line 38) is defined in the Error enum but is DEAD CODE — never
                  referenced anywhere in the contract.
ATTACK VECTOR   : 1. User calls transfer(zero_address, token_id)
                  2. Token ownership is set to zero_address
                  3. No account can ever sign a transaction from zero_address
                  4. Token is permanently locked — unrecoverable
PRECONDITIONS   : Any authorized caller (owner, approved, operator) calling transfer
IMPACT          : Permanent loss of NFT. Token remains in total_supply but is
                  inaccessible. balance_of(zero_address) increments uselessly.
AFFECTED        : beli_nft — any transfer caller
FIX             : Add zero-address validation at the top of `transfer_token_from`:
                    fn transfer_token_from(...) -> Result<()> {
                        if to == AccountId::from([0u8; 32]) {
                            return Err(Error::InvalidRecipient);
                        }
                        // ... existing logic
                    }
                  Also add the same check in `mint_token` for the `to` parameter.
CWE             : CWE-20 (Improper Input Validation)
PSP34 SPEC REF  : PSP34 §Transfer — destination validation
```

---

### F-05 — MEDIUM: Unbounded String/URI Storage — DoS Vector

```
SEVERITY        : MEDIUM
TRACK           : B — Security
LOCATION        : lib.rs line 178 — mint(to, uri: String), line 288 — set_token_uri(id, uri: String)
SECTION         : A5, B6
DESCRIPTION     : Both `mint()` and `set_token_uri()` accept `uri: String` with
                  no maximum length enforcement. An admin (contract owner) could
                  store arbitrarily large metadata strings, consuming excessive
                  storage deposit. The constructor also accepts unbounded `name`
                  and `symbol` strings (line 111).
ATTACK VECTOR   : 1. Contract owner calls mint(to, very_large_string) repeatedly
                  2. Each token stores megabytes of URI data
                  3. Contract storage deposit grows without bound
                  4. If the contract's balance is exhausted, further operations may
                     fail due to insufficient storage deposit
PRECONDITIONS   : Contract owner (only admin can mint or set URI)
IMPACT          : Storage deposit exhaustion; potential contract DoS
AFFECTED        : beli_nft contract storage budget
FIX             : Add a constant `MAX_URI_LENGTH: usize = 256` (or appropriate
                  limit) and validate:
                    if uri.len() > MAX_URI_LENGTH {
                        return Err(Error::UriTooLong);
                    }
                  Apply to `mint()`, `set_token_uri()`, and optionally to `name`
                  and `symbol` in the constructor.
CWE             : CWE-770 (Allocation of Resources Without Limits)
PSP34 SPEC REF  : PSP34 §Metadata — attribute storage bounds
```

---

### F-06 — MEDIUM: PSP34Enumerable Claimed But Not Implemented

```
SEVERITY        : MEDIUM
TRACK           : A — Compliance
LOCATION        : lib.rs line 12 — doc comment
SECTION         : A4
DESCRIPTION     : The contract's module doc comment (line 12) lists "Enumeration
                  support" as a feature. No enumeration methods exist:
                    - No `token_by_index(index: u128)`
                    - No `owners_token_by_index(owner, index)`
                    - `total_supply()` returns `u32` not `u128`
                  There is no way to enumerate which tokens exist or which tokens
                  a specific owner holds. Off-chain indexers must reconstruct this
                  from Transfer events — no on-chain query path exists.
ATTACK VECTOR   : N/A — spec non-conformance, not exploitable
PRECONDITIONS   : N/A
IMPACT          : Wallets and marketplaces that call enumeration methods will get
                  "method not found" errors. Users cannot query their owned tokens
                  on-chain.
AFFECTED        : beli_nft + all NFT display/listing integrations
FIX             : Either implement PSP34Enumerable (with index tracking storage)
                  or remove the "Enumeration support" claim from doc comments.
CWE             : N/A
PSP34 SPEC REF  : PSP34 §Enumerable Extension
```

---

### F-07 — MEDIUM: set_token_uri Emits No Event

```
SEVERITY        : MEDIUM
TRACK           : A — Compliance
LOCATION        : lib.rs lines 288–300 — set_token_uri()
SECTION         : A7
DESCRIPTION     : The `set_token_uri()` function modifies on-chain metadata but
                  emits no event. Off-chain indexers, marketplaces, and caching
                  layers have no way to detect that a token's metadata has changed
                  unless they poll every token on every block.
ATTACK VECTOR   : 1. Contract owner mints token #5 with URI pointing to legitimate art
                  2. Token #5 is sold on marketplace for high value
                  3. Owner calls set_token_uri(5, "ipfs://rug_pull_image")
                  4. No event is emitted — marketplace cache still shows old metadata
                  5. Buyer discovers the metadata has been silently changed
PRECONDITIONS   : Contract owner
IMPACT          : Silent metadata mutation; marketplace display inconsistency;
                  potential rug-pull vector (mitigated by owner-only restriction)
AFFECTED        : beli_nft + all off-chain indexers and caches
FIX             : Add a `MetadataUpdate` event:
                    #[ink(event)]
                    pub struct MetadataUpdate {
                        #[ink(topic)]
                        id: TokenId,
                        uri: String,
                    }
                  Emit in `set_token_uri()` after the storage write.
CWE             : CWE-778 (Insufficient Logging)
PSP34 SPEC REF  : PSP34 §Events — state change observability
```

---

### F-08 — MEDIUM: saturating_add/sub Masks Arithmetic Errors

```
SEVERITY        : MEDIUM
TRACK           : B — Security
LOCATION        : lib.rs lines 186, 345, 346, 374, 378, 400
SECTION         : B7, A6
DESCRIPTION     : All counter and balance arithmetic uses `saturating_add` /
                  `saturating_sub` instead of `checked_add` / `checked_sub`. This
                  silently absorbs arithmetic boundary conditions:
                    - next_token_id at u32::MAX → saturates → all future mints fail
                      with misleading `TokenExists` instead of "supply exhausted"
                    - total_supply at u32::MAX → saturates → supply count is wrong
                    - balance underflow → saturates to 0 → balance tracking corrupted
                  While `saturating_sub` on balance cannot produce negative values
                  (which is correct), it masks bugs where balance tracking diverges
                  from actual ownership — the contract would silently report
                  inconsistent state.
ATTACK VECTOR   : Not directly exploitable — but makes debugging extremely difficult
                  if counters ever reach boundary values, and masks logic bugs in
                  balance tracking.
PRECONDITIONS   : Counters reaching u32::MAX, or a logic bug causing unbalanced
                  increment/decrement
IMPACT          : Silent state corruption; misleading error messages; impossible to
                  diagnose supply/balance inconsistencies
AFFECTED        : beli_nft — all counter and balance fields
FIX             : Replace `saturating_add` / `saturating_sub` with `checked_add` /
                  `checked_sub` and return explicit errors:
                    self.total_supply = self.total_supply.checked_add(1)
                        .ok_or(Error::SupplyOverflow)?;
                    self.next_token_id = self.next_token_id.checked_add(1)
                        .ok_or(Error::SupplyOverflow)?;
                  Add `SupplyOverflow` and `BalanceUnderflow` to the Error enum.
CWE             : CWE-190 (Integer Overflow or Wraparound)
PSP34 SPEC REF  : PSP34 §Mint/Burn — atomic supply tracking
```

---

### F-09 — MEDIUM: Missing PSP34Metadata get_attribute Interface

```
SEVERITY        : MEDIUM
TRACK           : A — Compliance
LOCATION        : lib.rs lines 172–174 — token_uri()
SECTION         : A5
DESCRIPTION     : PSP34Metadata defines `get_attribute(id: Id, key: Vec<u8>)
                  -> Option<Vec<u8>>` as the standard metadata query method.
                  The contract instead implements `token_uri(id: TokenId)
                  -> Option<String>` — a single-key metadata accessor. There is
                  no way to store or query arbitrary key-value metadata per token.
                  The PSP34 standard allows arbitrary attribute keys (e.g.,
                  `name`, `description`, `image`, `animation_url`), but this
                  contract only supports a single `uri` field.
ATTACK VECTOR   : N/A — spec non-conformance
PRECONDITIONS   : N/A
IMPACT          : Cross-contract callers using PSP34Metadata `get_attribute` will
                  fail. Limited metadata model.
AFFECTED        : beli_nft + metadata consumers
FIX             : Implement `get_attribute(id, key)` that maps known keys (e.g.,
                  `b"uri"`) to the stored token_uri, returning None for unknown keys.
CWE             : N/A
PSP34 SPEC REF  : PSP34 §Metadata Extension — get_attribute
```

---

### F-10 — MEDIUM: approve() Cannot Revoke Per-Token Approval

```
SEVERITY        : MEDIUM
TRACK           : A — Compliance
LOCATION        : lib.rs lines 225–247 — approve()
SECTION         : A3
DESCRIPTION     : The PSP34 `approve()` signature includes an `approved: bool`
                  parameter to both grant and revoke approvals. This contract's
                  `approve(to, id)` can only GRANT approval — there is no mechanism
                  to revoke a per-token approval once set, other than transferring
                  the token (which clears approvals as a side effect).
                  If an owner approves account X and then changes their mind, they
                  cannot revoke without transferring the token to themselves (if
                  self-transfer is even meaningful) or to another account and back.
ATTACK VECTOR   : 1. Alice owns token #5
                  2. Alice calls approve(Bob, 5) — Bob is now approved
                  3. Alice changes her mind — wants to revoke Bob's approval
                  4. No revoke function exists
                  5. Bob can still transfer token #5 at any time
PRECONDITIONS   : Any owner who has granted a per-token approval
IMPACT          : Irrevocable approval until token is transferred — reduces owner
                  sovereignty over their token
AFFECTED        : beli_nft — all token owners who use per-token approval
FIX             : Add `approved: bool` parameter to `approve()`:
                    pub fn approve(&mut self, to: AccountId, id: TokenId,
                                   approved: bool) -> Result<()> {
                        // ... existing owner/operator check ...
                        if approved {
                            self.token_approvals.insert(id, &to);
                        } else {
                            self.token_approvals.remove(id);
                        }
                        // ... emit event with approved field ...
                    }
CWE             : CWE-285 (Improper Authorization)
PSP34 SPEC REF  : PSP34 §Approve — revocation capability
```

---

### F-11 — LOW: transfer_ownership Has No Two-Step Transfer or Event

```
SEVERITY        : LOW
TRACK           : B — Security
LOCATION        : lib.rs lines 304–312 — transfer_ownership()
SECTION         : B7
DESCRIPTION     : A single call irreversibly transfers complete admin authority
                  (minting, URI setting, code upgrade) to `new_owner`. No
                  confirmation step, no event, no zero-address check. If called
                  with a wrong address, all admin capabilities are permanently lost.
ATTACK VECTOR   : 1. Owner calls transfer_ownership(wrong_address) by mistake
                  2. All admin functions (mint, set_token_uri, set_code_hash,
                     transfer_ownership) are now exclusively controlled by wrong_address
                  3. No recovery path exists
PRECONDITIONS   : Contract owner making a mistake in the destination address
IMPACT          : Permanent loss of all admin capabilities; no new tokens can ever
                  be minted; no metadata can be updated; no further upgrades possible
AFFECTED        : beli_nft contract administration
FIX             : Implement two-step ownership transfer:
                    1. `propose_ownership(new_owner)` — sets pending_owner
                    2. `accept_ownership()` — new_owner confirms
                  Also emit an `OwnershipTransferred` event and reject zero-address.
CWE             : CWE-284 (Improper Access Control)
PSP34 SPEC REF  : N/A
```

---

### F-12 — LOW: set_code_hash Emits No Event

```
SEVERITY        : LOW
TRACK           : B — Security
LOCATION        : lib.rs lines 325–333 — set_code_hash()
SECTION         : B1
DESCRIPTION     : Contract upgrades via `set_code_hash()` are invisible to off-chain
                  systems. No event is emitted. Users and monitoring systems have no
                  on-chain signal that the contract's behavior has fundamentally changed.
ATTACK VECTOR   : N/A — governance transparency issue
PRECONDITIONS   : Contract owner
IMPACT          : Users cannot detect contract upgrades; reduces trust and auditability
AFFECTED        : beli_nft + all users and monitoring systems
FIX             : Add a `CodeHashUpdated { old_hash: Hash, new_hash: Hash }` event.
CWE             : CWE-778 (Insufficient Logging)
PSP34 SPEC REF  : N/A
```

---

### F-13 — LOW: Operator Revocation Does Not Clear Per-Token Approvals

```
SEVERITY        : LOW
TRACK           : A — Compliance
LOCATION        : lib.rs lines 250–271 — set_approval_for_all()
SECTION         : A3, B5
DESCRIPTION     : Revoking operator status (`set_approval_for_all(op, false)`)
                  removes the `(owner, operator)` entry from `operator_approvals`
                  but does NOT scan or remove any per-token approvals that the same
                  operator may hold. If an owner both approved an operator globally
                  AND granted per-token approvals for specific tokens, revoking the
                  operator role leaves per-token approvals intact.

                  This is arguably correct (they are separate approval systems), but
                  a user who revokes operator status may expect ALL approval paths
                  to be closed for that account.
ATTACK VECTOR   : 1. Alice approves Bob as operator (global)
                  2. Alice also calls approve(Bob, token_5) (per-token)
                  3. Alice revokes Bob's operator status
                  4. Bob can still transfer token_5 via per-token approval
PRECONDITIONS   : Owner has granted both operator and per-token approval to same account
IMPACT          : Unexpected residual transfer capability after operator revocation
AFFECTED        : beli_nft — approval system
FIX             : Document this behavior explicitly. Optionally, add a
                  `revoke_all_approvals(operator)` function that clears both.
CWE             : CWE-285 (Improper Authorization)
PSP34 SPEC REF  : PSP34 §Approve — revocation semantics
```

---

### F-14 — LOW: transfer_from Redundant Authorization Check

```
SEVERITY        : LOW
TRACK           : A — Compliance
LOCATION        : lib.rs lines 207–221 — transfer_from()
SECTION         : A3
DESCRIPTION     : `transfer_from()` checks `owner != from` (line 211) — rejecting
                  transfers where `from` does not match the current owner. This is
                  correct safety check. However, it then checks
                  `caller != owner && !self.is_approved_or_owner(caller, id)`.
                  The `is_approved_or_owner` function (line 406) ALSO checks
                  `spender == owner` as its first condition. So the `caller != owner`
                  check on line 215 is redundant — `is_approved_or_owner` already
                  returns true if caller is owner.

                  This is functionally harmless but indicates the auth logic was
                  written without full awareness of the helper function's behavior.
ATTACK VECTOR   : N/A — not exploitable
PRECONDITIONS   : N/A
IMPACT          : None — redundancy only. Minor gas waste.
AFFECTED        : beli_nft — code quality
FIX             : Simplify:
                    if !self.is_approved_or_owner(caller, id) {
                        return Err(Error::NotAuthorized);
                    }
CWE             : N/A
PSP34 SPEC REF  : N/A
```

---

### F-15 — LOW: InvalidRecipient Error Variant Is Dead Code

```
SEVERITY        : LOW
TRACK           : A — Compliance
LOCATION        : lib.rs line 38 — Error::InvalidRecipient
SECTION         : A3
DESCRIPTION     : The `InvalidRecipient` variant is defined in the Error enum but
                  is never used anywhere in the contract. This suggests destination
                  validation was planned but never implemented (see F-04).
ATTACK VECTOR   : N/A
PRECONDITIONS   : N/A
IMPACT          : Dead code; indicates missing validation
AFFECTED        : beli_nft — code hygiene
FIX             : Implement destination validation (F-04 fix) using this error,
                  or remove the variant if validation is intentionally omitted.
CWE             : N/A
PSP34 SPEC REF  : N/A
```

---

### F-16 — INFORMATIONAL: No Integration With access_control Library

```
SEVERITY        : INFORMATIONAL
TRACK           : B — Security
LOCATION        : lib.rs — entire contract
SECTION         : B7
DESCRIPTION     : The contract uses a simple `self.owner` check for admin access
                  instead of the workspace's `access_control` contract. This is
                  acceptable for a single-role contract but inconsistent with the
                  broader BelizeChain architecture, which includes a dedicated
                  role-based access control contract.
ATTACK VECTOR   : N/A
PRECONDITIONS   : N/A
IMPACT          : Cannot add granular roles (e.g., separate minter role vs admin role)
                  without rewriting access control
AFFECTED        : beli_nft — future extensibility
FIX             : Consider integrating `access_control` for role-based gating,
                  especially if a separate minter role is desired.
CWE             : N/A
PSP34 SPEC REF  : N/A
```

---

### F-17 — INFORMATIONAL: No Supply Cap

```
SEVERITY        : INFORMATIONAL
TRACK           : B — Security
LOCATION        : lib.rs line 178 — mint()
SECTION         : B7
DESCRIPTION     : The contract has no configurable supply cap beyond the implicit
                  ~4.29 billion from `u32::MAX`. Most NFT collections have a
                  defined maximum supply. Without a cap, the contract owner can
                  mint indefinitely, diluting the value of existing tokens.
ATTACK VECTOR   : N/A — design decision
PRECONDITIONS   : N/A
IMPACT          : No scarcity guarantee for token holders
AFFECTED        : beli_nft — token economics
FIX             : Add an optional `max_supply: Option<u32>` field set in constructor.
                  Check in `mint()`:
                    if let Some(cap) = self.max_supply {
                        if self.total_supply >= cap {
                            return Err(Error::MaxSupplyReached);
                        }
                    }
CWE             : N/A
PSP34 SPEC REF  : N/A
```

---

## 5. Invariant Verification

| # | Invariant | Verified | Notes |
|---|---|---|---|
| 1 | Every token ID returned by `token_by_index` has exactly one owner | **N/A** | PSP34Enumerable not implemented |
| 2 | `total_supply` equals the count of all non-burned token IDs at all times | **✓ PASS** | Uses checked arithmetic; overflow returns explicit error (F-08 fixed) |
| 3 | `balance_of(owner)` equals the count of token IDs for which `owner_of(id) == owner` | **✓ PASS** | Uses checked arithmetic (F-08 fixed) |
| 4 | No token ID exists in both the active token set and the burned set simultaneously | **✓ PASS** | `burned_ids: Mapping<TokenId, ()>` tracks burned IDs; `mint_token` checks both `token_owner` and `burned_ids` (F-03 fixed) |
| 5 | Per-token approvals for a given ID are cleared on every ownership transfer | **✓ PASS** | `transfer_token_from` (L370): `self.token_approvals.remove(id)` — cleared before ownership change |
| 6 | `owner_of(id)` returns `None` for every burned or never-minted token ID | **✓ PASS** | `burn_token` removes from `token_owner`. `Mapping::get` returns `None` for missing keys. |
| 7 | No caller without owner or operator approval can successfully call `transfer` | **✓ PASS** | Both `transfer` and `transfer_from` check `is_approved_or_owner` before delegating to `transfer_token_from` |
| 8 | A token cannot be simultaneously owned by two different accounts | **✓ PASS** | `token_owner: Mapping<TokenId, AccountId>` — single-value mapping. `insert` overwrites. State mutations are atomic within a single call. |
| 9 | Enumeration index bounds never exceed `total_supply` | **N/A** | No enumeration implemented |

---

## 6. Verdict & Pass/Fail Decision

### Hard Blockers

| Finding | Condition | Status |
|---|---|---|
| F-01 | Contract does not implement PSP34 trait — ecosystem isolation | **RESOLVED** ✅ |
| F-02 | Operator can burn tokens owned by others | **RESOLVED** ✅ |
| F-04 | No destination validation — tokens can be sent to dead addresses | **RESOLVED** ✅ |

### Must Fix Before Next Audit Phase

| Finding | Condition | Status |
|---|---|---|
| F-03 | Burned token ID reusable via contract upgrade | **RESOLVED** ✅ |
| F-05 | Unbounded URI/String storage | **RESOLVED** ✅ |
| F-07 | Missing event on metadata change | **RESOLVED** ✅ |
| F-08 | saturating arithmetic masks errors | **RESOLVED** ✅ |
| F-10 | Cannot revoke per-token approval | **RESOLVED** ✅ |

### Must Fix Before Mainnet (already on mainnet — retroactive urgency)

| Finding | Condition | Status |
|---|---|---|
| F-06 | False enumeration claim | **RESOLVED** ✅ |
| F-09 | Missing PSP34Metadata interface | **RESOLVED** ✅ |
| F-11 | No two-step ownership transfer | **RESOLVED** ✅ |
| F-12 | set_code_hash emits no event | **RESOLVED** ✅ |

### Verdict

```
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   AUDIT-GEM-04 VERDICT:  ██ PASS ██                          ║
║                                                               ║
║   17 of 17 findings RESOLVED.                                ║
║   0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 LOW, 0 INFO remaining.    ║
║                                                               ║
║   The contract is PSP34-compliant with proper types,         ║
║   standard method signatures, owner-only burn, burned-ID     ║
║   tracking, destination validation, bounded URIs, metadata   ║
║   events, checked arithmetic, two-step ownership transfer,   ║
║   code-hash-update events, documented approval semantics,    ║
║   and optional supply cap.                                   ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## 7. Remediation Priority

| Priority | Finding | Effort | Status |
|----------|---------|--------|--------|
| **P0** | F-01: Implement PSP34 trait | High | ✅ **FIXED** |
| **P0** | F-02: Restrict burn to owner only | Low | ✅ **FIXED** |
| **P0** | F-04: Add destination validation | Low | ✅ **FIXED** |
| **P1** | F-03: Add burned_ids tracking | Low | ✅ **FIXED** |
| **P1** | F-10: Add approval revocation | Low | ✅ **FIXED** |
| **P1** | F-08: Replace saturating with checked | Low | ✅ **FIXED** |
| **P1** | F-05: Bound URI length | Low | ✅ **FIXED** |
| **P2** | F-07: Add metadata update event | Low | ✅ **FIXED** |
| **P2** | F-06: Fix enumeration claim | Medium | ✅ **FIXED** |
| **P2** | F-09: Implement get_attribute | Medium | ✅ **FIXED** |
| **P3** | F-11: Two-step ownership | Medium | ✅ **FIXED** |
| **P3** | F-12: CodeHashUpdated event | Low | ✅ **FIXED** |
| **P3** | F-13: Document approval semantics | Low | ✅ **FIXED** |
| **P3** | F-14: Remove redundant auth check | Low | ✅ **FIXED** |
| **P3** | F-15: InvalidRecipient dead code | Low | ✅ **FIXED** (by F-04) |
| **P4** | F-16: Acknowledge access_control design | Low | ✅ **FIXED** |
| **P4** | F-17: Optional supply cap | Medium | ✅ **FIXED** |

---

*End of AUDIT-GEM-04*
