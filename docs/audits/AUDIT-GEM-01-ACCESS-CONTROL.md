# AUDIT-GEM-01 — Access Control Library Security Audit

| Field | Value |
|---|---|
| **Audit ID** | AUDIT-GEM-01 |
| **Target** | `access_control/lib.rs` (630 lines) |
| **Scope** | Ownable, RBAC, Pausable patterns; consumer integration |
| **Framework** | ink! 5.1.1 / Substrate pallet-contracts |
| **Cargo deps** | `parity-scale-codec 3.7.5`, `scale-info 2.11.6` |
| **Status** | **FAIL — Hard Blockers Present** |
| **Date** | 2025-07 |

---

## Executive Summary

The `access_control` library implements three foundational security patterns — Ownable, RBAC, and Pausable — for the GEM smart contract platform on BelizeChain. The library is architecturally structured as a "passive" data module: all functions accept an explicit `caller: AccountId` parameter and never call `self.env().caller()` internally. This design places the entire burden of correct caller forwarding on consuming contracts.

**Three CRITICAL vulnerabilities were identified in the library itself.** The Pausable module (`PausableData::pause` and `PausableData::unpause`) contains zero access control: the `caller` argument is used exclusively for event emission and is never checked against any authorization predicate. The RBAC module's `renounce_role` function accepts an arbitrary `caller: AccountId` with no enforcement that the caller is the actual transaction signer — a consuming contract could pass any account ID and renounce that account's role without consent.

**A fourth finding of equal effective severity** is structural rather than code-level: **no consuming contract in the GEM workspace imports or uses this library.** Every contract (`dalla_token`, `beli_nft`, `psp37_multi_token`, `simple_dao`, `faucet`, `dex/factory`, `dex/pair`, `dex/router`) implements its own siloed, inconsistent, unaudited inline access control. The audit brief's statement that "every GEM contract imports this library" is demonstrably false. One contract, `dex/factory`, has no access control at all.

**This audit fails.** The library cannot advance to production use and no downstream contract audit should proceed until all Hard Blockers are resolved. Remediation guidance is provided for every finding.

**Summary table:**

| Severity | Count |
|---|---|
| CRITICAL | 3 |
| HIGH | 4 |
| MEDIUM | 3 |
| LOW | 3 |
| INFORMATIONAL | 4 |
| **TOTAL** | **17** |

---

## Pass / Fail Assessment

| Condition | Result |
|---|---|
| Any code path allowing unauthorized ownership assumption | **PASS** |
| Any code path allowing unauthorized role grant or escalation | **PASS** |
| Single-step ownership transfer with no confirmation | **FAIL — Hard Blocker (Finding GEM-01-H07)** |
| `unpause` callable by any account other than designated role | **FAIL — Hard Blocker (Finding GEM-01-C01)** |
| Any modifier that can be bypassed via trait override | **PASS** (library has no traits with default implementations) |
| Any internal privileged function exposed as `pub` | **PASS** (all internals are `pub(crate)` or private) |
| Uninitialized storage fields that default to granting access | **PASS** |
| Missing event on any access control state change | **PASS** (all state changes emit events via closure) |
| `unwrap()` or `expect()` in any access check code path | **PASS** (test-only `unwrap()` at L598/618/624 — not production) |
| Storage layout undocumented or unstable | **FAIL — Must Fix Before Mainnet (Finding GEM-01-I14)** |
| Consuming contract misuse patterns identified in any GEM contract | **FAIL — Must Fix Before Mainnet (Findings GEM-01-H04, GEM-01-M10)** |

**Overall verdict: FAIL. Three hard blockers require resolution before any downstream use.**

---

## Findings

---

### GEM-01-C01 — `PausableData::unpause` Has No Access Control

```
SEVERITY       : CRITICAL
LOCATION       : lib.rs line 448 — PausableData::unpause
PATTERN        : Pausable
```

**DESCRIPTION:**  
`PausableData::unpause` accepts a `caller: AccountId` argument, but that argument is passed directly to the `emit_event` closure and is never checked against any authorization predicate. The only enforcement inside the function body is `self.ensure_paused()` — a state consistency check, not an access control check. Any consuming contract that calls `pausable.unpause(caller, ...)` without a prior authorization gate at the ink! message boundary will allow any transaction signer to unpause the contract.

The relevant code:

```rust
// lib.rs L448 (approximate)
pub fn unpause(&mut self, caller: AccountId, emit_event: impl FnOnce(Unpaused)) -> Result<(), AccessError> {
    self.ensure_paused()?;
    self.paused = false;
    emit_event(Unpaused { account: caller });
    Ok(())
}
```

The `caller` value is used solely to populate the event struct. There is no `ensure_role`, `ensure_owner`, or any other check on `caller` anywhere in this function.

**ATTACK VECTOR:**  
1. A contract integrates `PausableData` and exposes `fn unpause(&mut self)` as a public ink! message.
2. The message passes `self.env().caller()` as the `caller` argument to `pausable.unpause(...)` without performing any prior role or ownership check.
3. Any external account calls the contract's `unpause` message.
4. `PausableData::unpause` executes, sets `paused = false`, and emits `Unpaused` with the attacker's address.
5. Emergency pause is defeated. All state-mutating functions gated by `ensure_not_paused` are re-enabled.

**PRECONDITIONS:**  
- At least one consuming contract integrates `PausableData`.
- That consuming contract exposes an `unpause` message that delegates without enforcing caller authorization.
- The contract is currently paused (typically meaning an emergency is in progress).

**IMPACT:**  
Denial of emergency-pause protection. An attacker can cancel an emergency pause initiated by the protocol team, re-enabling all paused operations (withdrawals, transfers, swaps, minting) during an active exploit or insolvency event. This is the highest-risk outcome of the Pausable pattern — pausing is a safety primitive; defeating it under adversarial conditions can cause total loss of funds across affected contracts.

**AFFECTED:**  
All GEM contracts that integrate `PausableData`. At present, no GEM contract imports this library (see GEM-01-H04), so exposure is zero — but the vulnerability is a prerequisite blocker for any future integration.

**FIX:**  
Add an authorization check inside `PausableData::unpause` itself, or enforce that consuming contracts must provide an authorization closure. The safest approach is an additional `authorized: bool` parameter:

```rust
pub fn unpause(
    &mut self,
    caller: AccountId,
    authorized: bool,               // ← consuming contract resolves this
    emit_event: impl FnOnce(Unpaused),
) -> Result<(), AccessError> {
    if !authorized {
        return Err(AccessError::MissingRole);
    }
    self.ensure_paused()?;
    self.paused = false;
    emit_event(Unpaused { account: caller });
    Ok(())
}
```

The consuming contract then enforces authorization before calling:

```rust
#[ink(message)]
pub fn unpause(&mut self) -> Result<(), AccessError> {
    let caller = self.env().caller();
    let authorized = self.access_control.has_role(PAUSER_ROLE, caller);
    self.pausable.unpause(caller, authorized, |event| self.env().emit_event(event))
}
```

Alternatively, accept an `auth_fn: impl Fn(AccountId) -> bool` closure argument and invoke it inside `unpause`.

**CWE:** CWE-862 (Missing Authorization)

---

### GEM-01-C02 — `PausableData::pause` Has No Access Control

```
SEVERITY       : CRITICAL
LOCATION       : lib.rs line 435 — PausableData::pause
PATTERN        : Pausable
```

**DESCRIPTION:**  
Identical issue to GEM-01-C01. `PausableData::pause` accepts `caller: AccountId` and uses it only for event emission. No authorization check occurs inside the function. Any account can trigger a pause on any consuming contract that fails to apply an external guard. Rated CRITICAL rather than HIGH because pausing is a privileged state change that misrepresents the contract's operational status on-chain, and griefing attacks using unauthorized pause calls can be used as a DoS mechanism.

```rust
// lib.rs L435 (approximate)
pub fn pause(&mut self, caller: AccountId, emit_event: impl FnOnce(Paused)) -> Result<(), AccessError> {
    self.ensure_not_paused()?;
    self.paused = true;
    emit_event(Paused { account: caller });
    Ok(())
}
```

**ATTACK VECTOR:**  
1. Consuming contract exposes a `pause` message with no prior authorization check.
2. Any account calls the message.
3. The DEX, faucet, or token contract is paused by an unprivileged actor, halting all operations (DoS).

**PRECONDITIONS:**  
- Consuming contract integrates `PausableData` and exposes `pause` without an external guard.

**IMPACT:**  
Griefing / DoS. An attacker can permanently cycle a contract between paused and unpaused states, prevent withdrawals, or disrupt a live swap operation. Combined with GEM-01-C01, any account can pause and unpause freely.

**AFFECTED:**  
All GEM contracts that integrate `PausableData` (none at present — see GEM-01-H04).

**FIX:**  
Apply the same `authorized: bool` parameter pattern as described in GEM-01-C01. For `pause`, the authorized role is typically `PAUSER_ROLE`.

**CWE:** CWE-862 (Missing Authorization)

---

### GEM-01-C03 — `renounce_role` Does Not Enforce Caller Identity

```
SEVERITY       : CRITICAL
LOCATION       : lib.rs line 325 — AccessControlData::renounce_role
PATTERN        : RBAC
```

**DESCRIPTION:**  
`AccessControlData::renounce_role` accepts a `caller: AccountId` argument and removes `role` from `caller`'s role set. The function contains no check that `caller == self.env().caller()` (i.e., that the supplied `caller` is actually the transaction signer). A consuming contract that passes an attacker-supplied value or any AccountId other than `self.env().caller()` will renounce that account's role without that account's consent.

The field comment or documentation does not state this enforcement is delegated to the consumer. The conventional semantic of "renounce" in RBAC is that an account voluntarily relinquishes its own role — it must be impossible for a third party to force renouncement.

**ATTACK VECTOR:**  
1. Consuming contract exposes `fn renounce_role(&mut self, role: RoleType, account: AccountId)` as a public ink! message — a plausible design if the developer misunderstands the library's trust model.
2. Attacker calls `renounce_role(PAUSER_ROLE, victim_account)`.
3. The consuming contract passes `account` as the `caller` argument to `access_control.renounce_role(account, role, ...)`.
4. The PAUSER_ROLE is stripped from `victim_account` without `victim_account` signing the transaction.
5. Attacker repeats for all role holders of critical roles, including DEFAULT_ADMIN_ROLE.

**PRECONDITIONS:**  
- Consuming contract passes a caller-controlled or storage-read `AccountId` as the `caller` argument rather than `self.env().caller()`.

**IMPACT:**  
Forced role renouncement for all role types, including DEFAULT_ADMIN_ROLE. An attacker can strip all admins, rendering the contract permanently ungovernable. Can be used to escalate privilege: strip all PAUSER_ROLE holders, then exploit an unpaused contract; strip UPGRADER_ROLE holders to prevent patching after an exploit.

**AFFECTED:**  
All GEM contracts that integrate `AccessControlData` (none at present — see GEM-01-H04).

**FIX:**  
Add an explicit caller identity check inside the library function and remove the flexibility for consuming contracts to override it:

```rust
pub fn renounce_role(
    &mut self,
    env_caller: AccountId,   // must be self.env().caller() at the call site
    role: RoleType,
    account: AccountId,
    emit_event: impl FnOnce(RoleRevoked),
) -> Result<(), AccessError> {
    // Enforce: only the actual signer can renounce their own role
    if env_caller != account {
        return Err(AccessError::InvalidCaller);
    }
    // ... existing revocation logic
}
```

The consuming contract must always pass `self.env().caller()` as `env_caller`. Document this constraint explicitly. Consider renaming the parameter to `env_caller` to signal its required origin.

**CWE:** CWE-284 (Improper Access Control), CWE-306 (Missing Authentication for Critical Function)

---

### GEM-01-H04 — Library Is Unused; All Consumers Implement Siloed Inline Access Control

```
SEVERITY       : HIGH
LOCATION       : Workspace-level (all consumer Cargo.toml files)
PATTERN        : Misuse Surface
```

**DESCRIPTION:**  
The audit brief states: *"Every GEM contract imports this library. A vulnerability here is a vulnerability everywhere."* This premise is factually incorrect. Grep analysis of all `Cargo.toml` dependency declarations and `use` statements across the workspace confirms that **zero GEM contracts list `access_control` as a dependency or import any type from it.**

Each contract has implemented its own ad hoc access control:

| Contract | Pattern Used | Notes |
|---|---|---|
| `faucet` | `owner: AccountId`, inline `ensure_owner()` at L265–267 | No zero-address guard at init |
| `dalla_token` | `owner: AccountId`, inline guard at message boundary | Non-standard pattern |
| `beli_nft` | `owner: AccountId`, own `owner_of` logic | Non-standard pattern |
| `psp37_multi_token` | `owner: AccountId`, inline `if caller != self.owner` at L288 | Non-standard pattern |
| `simple_dao` | `admin: AccountId` (not even `owner`), inline `if caller != self.admin` at L330/360, own `transfer_admin` at L358 | Naming inconsistency |
| `dex/factory` | **No access control found at all** | No owner, admin, role, or pause |
| `dex/pair` | Custom reentrancy lock (`locked: bool`, L72/L199) | Unrelated to Pausable — manually reset |
| `dex/router` | Deadline guard `_ensure_not_expired` only | No ownership or role-based AC |

This fragmentation means: there is no shared security baseline, no consistent event schema, no guaranteed zero-address validation, no pause mechanism on any DEX component, and the library built to address all of these is entirely unused.

The separate `dex/factory` finding (no access control at all) is independently worthy of HIGH classification but is reported here as part of the structural finding.

**ATTACK VECTOR:**  
- `dex/factory`: Protocol factory functions (pair creation, fee setting) have no access control. Any account can call privileged factory operations.
- Fragmented patterns: auditors of consuming contracts cannot rely on the shared library's guarantees — each contract must be independently audited from scratch against its own ad hoc pattern.

**PRECONDITIONS:**  
- `dex/factory` privileged functions are deployed as-is.

**IMPACT:**  
- `dex/factory`: Any account can potentially create unauthorized pairs, modify factory parameters, or perform admin operations (impact depends on which messages are exposed without guards).
- All contracts: Inconsistent patterns increase the probability of per-contract access control bugs surviving to production.
- Audit scope and cost: every contract requires a full independent access control audit; the shared library provides no reduction in attack surface.

**AFFECTED:**  
All GEM contracts. `dex/factory` is immediately exposed.

**FIX:**  
Two remediation paths, ordered by preference:

1. **Integrate the library** (preferred after library issues are fixed): Add `access_control` as a workspace dependency, replace per-contract inline ownership with `OwnableData`, and add `PausableData` to DEX contracts. Establish a single integration pattern documented in `docs/guides/CONTRIBUTING.md`.

2. **Standardize inline patterns** (acceptable interim): Define a mandatory per-contract checklist: zero-address init guard, two-step ownership transfer, consistent event emission, and `dex/factory` must add an owner or role check on all admin messages immediately.

**CWE:** CWE-710 (Improper Adherence to Coding Standards), CWE-284 (Improper Access Control) for `dex/factory`

---

### GEM-01-H05 — `revoke_role` Has No Last-Admin Guard

```
SEVERITY       : HIGH
LOCATION       : lib.rs line 299 — AccessControlData::revoke_role
PATTERN        : RBAC
```

**DESCRIPTION:**  
`revoke_role` correctly checks that the `caller` holds the admin role for the target `role`. However, there is no guard preventing the last holder of `DEFAULT_ADMIN_ROLE` from revoking their own DEFAULT_ADMIN_ROLE. An administrator who does so renders the contract permanently ungovernable: no account can grant or revoke any role, no role admin changes can be made, and any role-gated upgrades or emergency functions become permanently inaccessible.

This is not a theoretical risk. The same admin can call `revoke_role(caller, DEFAULT_ADMIN_ROLE, caller, ...)` in a single transaction.

**ATTACK VECTOR:**  
1. Single DEFAULT_ADMIN_ROLE holder calls `revoke_role(self, DEFAULT_ADMIN_ROLE, self)`.
2. `DEFAULT_ADMIN_ROLE` is cleared.
3. No account can call `grant_role` or `set_role_admin` because both require the admin role for the target role, and DEFAULT_ADMIN_ROLE (which is its own admin) has no holders.
4. Contract governance is permanently frozen.

**PRECONDITIONS:**  
- Only one account holds DEFAULT_ADMIN_ROLE.
- That account (or an attacker who compromises it) calls `revoke_role` on itself.

**IMPACT:**  
Permanent governance lockout. No new roles can be granted. Emergency operations requiring PAUSER_ROLE or UPGRADER_ROLE cannot be authorized if those roles are also vacant. Contract becomes immutable by administrative action.

**AFFECTED:**  
All consumers of `AccessControlData`.

**FIX:**  
A last-admin guard in `revoke_role`:

```rust
pub fn revoke_role(
    &mut self,
    caller: AccountId,
    role: RoleType,
    account: AccountId,
    emit_event: impl FnOnce(RoleRevoked),
) -> Result<(), AccessError> {
    let admin_role = self.get_role_admin(role);
    self.ensure_has_role(admin_role, caller)?;

    // Prevent revoking the last DEFAULT_ADMIN_ROLE holder
    if role == DEFAULT_ADMIN_ROLE {
        // Check that there is more than one DEFAULT_ADMIN_ROLE holder.
        // Since Mapping does not expose iteration, require the contract to
        // track admin count in a separate storage field.
        // Alternatively: disallow self-revocation for DEFAULT_ADMIN_ROLE entirely.
        if caller == account {
            return Err(AccessError::CannotRevokeLastAdmin);
        }
    }

    // ... existing revocation
}
```

A complete solution requires an admin-count counter updated on grant/revoke. The simple interim fix is to prohibit self-revocation of DEFAULT_ADMIN_ROLE.

**CWE:** CWE-269 (Improper Privilege Management)

---

### GEM-01-H06 — `renounce_ownership` Is Irrevocable with No Confirmation Parameter

```
SEVERITY       : HIGH
LOCATION       : lib.rs line 161 — OwnableData::renounce_ownership
PATTERN        : Ownable
```

**DESCRIPTION:**  
`OwnableData::renounce_ownership` sets `self.owner = None` and is protected only by `ensure_owner`. There is no confirmation parameter (e.g., a passphrase string, explicit boolean, or a typed enum that the caller must supply) to prevent accidental calls. Once `renounce_ownership` is called, the contract has no owner, ownership can never be restored, and all `only_owner` protected functions become permanently inaccessible.

A single misclick, front-end bug, or phishing transaction is sufficient to trigger this permanently.

**ATTACK VECTOR:**  
- Owner account is phished or front-end sends wrong transaction.
- `renounce_ownership` executes; `self.owner = None`.
- All privileged operations are permanently bricked.

**PRECONDITIONS:**  
- Caller is the current owner.

**IMPACT:**  
Permanent loss of contract ownership. All owner-gated functions (including upgrade, fee setting, pause, minting) become permanently inaccessible. No recovery path exists without contract redeployment.

**AFFECTED:**  
All consumers of `OwnableData`.

**FIX:**  
Require an explicit confirmation input:

```rust
pub fn renounce_ownership(
    &mut self,
    caller: AccountId,
    confirm: bool,               // caller must pass `true` explicitly
    emit_event: impl FnOnce(OwnershipTransferred),
) -> Result<(), AccessError> {
    self.ensure_owner(caller)?;
    if !confirm {
        return Err(AccessError::ConfirmationRequired);
    }
    let old_owner = self.owner;
    self.owner = None;
    emit_event(OwnershipTransferred { old_owner, new_owner: None });
    Ok(())
}
```

Additionally, consider a two-step pattern with a time delay matching the ownership transfer pattern.

**CWE:** CWE-693 (Protection Mechanism Failure)

---

### GEM-01-H07 — `transfer_ownership` Is Single-Step with No Confirmation

```
SEVERITY       : HIGH
LOCATION       : lib.rs line 134 — OwnableData::transfer_ownership
PATTERN        : Ownable
```

**DESCRIPTION:**  
`OwnableData::transfer_ownership` immediately sets the new owner in a single step. If the wrong address is provided (typo, zero address bypass failure, misconfigured front-end, clipboard hijack), ownership is permanently transferred to an uncontrolled account. The zero-address check is correctly implemented, but that guards only against one specific invalid input — any other invalid address silently succeeds.

The industry-standard mitigation for single-step transfer risk is a two-step propose-and-accept pattern: the current owner proposes a new owner, and only after the proposed owner explicitly accepts does the transfer complete.

**ATTACK VECTOR:**  
1. Owner calls `transfer_ownership(caller, wrong_address, ...)` — wrong address due to front-end bug or clipboard hijack.
2. Ownership immediately transfers to `wrong_address`.
3. Current owner loses all privileged access permanently.

**PRECONDITIONS:**  
- Caller is current owner.
- Any address that is not `AccountId::default()` is accepted without further validation.

**IMPACT:**  
Permanent loss of ownership to an uncontrolled account. All owner-gated functions are now accessible to the unintended recipient (or inaccessible if the key is lost). This is a Hard Blocker per the audit pass/fail criteria.

**AFFECTED:**  
All consumers of `OwnableData`.

**FIX:**  
Implement a two-step transfer:

```rust
// Step 1: propose
pub fn propose_ownership(
    &mut self,
    caller: AccountId,
    proposed: AccountId,
) -> Result<(), AccessError> {
    self.ensure_owner(caller)?;
    if proposed == AccountId::from([0u8; 32]) {
        return Err(AccessError::ZeroAddress);
    }
    self.pending_owner = Some(proposed);
    Ok(())
}

// Step 2: accept (called by the proposed new owner)
pub fn accept_ownership(
    &mut self,
    caller: AccountId,
    emit_event: impl FnOnce(OwnershipTransferred),
) -> Result<(), AccessError> {
    match self.pending_owner {
        Some(p) if p == caller => {
            let old_owner = self.owner;
            self.owner = Some(caller);
            self.pending_owner = None;
            emit_event(OwnershipTransferred { old_owner, new_owner: Some(caller) });
            Ok(())
        }
        _ => Err(AccessError::NotPendingOwner),
    }
}
```

Add `pending_owner: Option<AccountId>` to `OwnableData` storage.

**CWE:** CWE-287 (Improper Authentication)

---

### GEM-01-M08 — `set_role_admin` Allows Privilege Hierarchy Inversion on `DEFAULT_ADMIN_ROLE`

```
SEVERITY       : MEDIUM
LOCATION       : lib.rs line 348 — AccessControlData::set_role_admin
PATTERN        : RBAC
```

**DESCRIPTION:**  
`set_role_admin` requires the caller to hold `DEFAULT_ADMIN_ROLE` but does not constrain which `role` or `new_admin_role` values are passed. It is therefore possible to call `set_role_admin(caller, DEFAULT_ADMIN_ROLE, MINTER_ROLE)`, which sets DEFAULT_ADMIN_ROLE's governing admin to `MINTER_ROLE`. If all MINTER_ROLE holders are subsequently revoked, DEFAULT_ADMIN_ROLE has no admin that can govern it (since the admin of DEFAULT_ADMIN_ROLE would be MINTER_ROLE and there are no MINTER_ROLE holders). This creates a privilege hierarchy inversion that permanently degrades governance.

**ATTACK VECTOR:**  
1. Attacker compromises a DEFAULT_ADMIN_ROLE account.
2. Calls `set_role_admin(attacker, DEFAULT_ADMIN_ROLE, BURNER_ROLE)`.
3. Revokes all BURNER_ROLE holders.
4. DEFAULT_ADMIN_ROLE now references an empty role as its admin. No account can change DEFAULT_ADMIN_ROLE's configuration.

**PRECONDITIONS:**  
- Attacker controls a DEFAULT_ADMIN_ROLE account.

**IMPACT:**  
Governance deadlock specific to DEFAULT_ADMIN_ROLE. Exploitation requires elevated privilege (DEFAULT_ADMIN_ROLE) but the resulting state is irrecoverable.

**AFFECTED:**  
All consumers of `AccessControlData`.

**FIX:**  
Prevent DEFAULT_ADMIN_ROLE's admin from being changed to anything other than itself:

```rust
pub fn set_role_admin(/* ... */) -> Result<(), AccessError> {
    self.ensure_has_role(DEFAULT_ADMIN_ROLE, caller)?;
    // DEFAULT_ADMIN_ROLE must always govern itself
    if role == DEFAULT_ADMIN_ROLE && new_admin_role != DEFAULT_ADMIN_ROLE {
        return Err(AccessError::CannotChangeDefaultAdminRole);
    }
    // ... existing logic
}
```

**CWE:** CWE-269 (Improper Privilege Management)

---

### GEM-01-M09 — `grant_role` Allows Unconstrained `DEFAULT_ADMIN_ROLE` Proliferation

```
SEVERITY       : MEDIUM
LOCATION       : lib.rs line 272 — AccessControlData::grant_role
PATTERN        : RBAC
```

**DESCRIPTION:**  
`DEFAULT_ADMIN_ROLE` is its own admin role (`role_admins[DEFAULT_ADMIN_ROLE] = DEFAULT_ADMIN_ROLE`). Therefore, any account holding `DEFAULT_ADMIN_ROLE` can grant `DEFAULT_ADMIN_ROLE` to an arbitrary new account with zero constraints. There is no maximum admin count, no multi-signature requirement, and no time delay. An admin acting in bad faith (or a compromised key) can silently replicate the highest-privilege role to attacker-controlled accounts before being detected.

**ATTACK VECTOR:**  
1. Attacker compromises or social-engineers one DEFAULT_ADMIN_ROLE key.
2. Grants DEFAULT_ADMIN_ROLE to 10 attacker-controlled accounts.
3. Original key is revoked, but attacker now has permanent admin control with 10 fallback accounts.

**PRECONDITIONS:**  
- Attacker controls one DEFAULT_ADMIN_ROLE key, even temporarily.

**IMPACT:**  
Permanent escalation of privilege. Attacker can grant/revoke all roles, pause/unpause contracts, upgrade contracts, mint or burn tokens indefinitely.

**AFFECTED:**  
All consumers of `AccessControlData`.

**FIX:**  
This is a known design tradeoff in RBAC systems. Recommended mitigations (in priority order):
1. Implement a timelock on `DEFAULT_ADMIN_ROLE` grants — the grant is queued and only takes effect after a delay, allowing monitoring.
2. Require multi-signature for DEFAULT_ADMIN_ROLE operations (out of scope for this library, but document the requirement).
3. Emit a high-visibility `DefaultAdminRoleGranted` event distinct from the generic `RoleGranted` event to aid monitoring.

**CWE:** CWE-269 (Improper Privilege Management)

---

### GEM-01-M10 — Siloed per-Contract Access Control Creates Inconsistent Security Properties

```
SEVERITY       : MEDIUM
LOCATION       : All consuming contracts (workspace-level finding)
PATTERN        : Misuse Surface
```

**DESCRIPTION:**  
Each contract in the GEM workspace implements its own independent inline access control with no shared baseline. The resulting inconsistencies include:

- `faucet`, `dalla_token`, `beli_nft`, `psp37_multi_token` use `owner: AccountId`; `simple_dao` uses `admin: AccountId` — different naming for the same concept, with no semantic alignment.
- `faucet::ensure_owner` (L265) calls `self.env().caller()` internally — correct; but the pattern is not enforced elsewhere.
- `simple_dao::transfer_admin` (L358): sets `self.admin = new_admin` with no zero-address guard.
- `faucet::transfer_ownership` (L162): sets `self.owner = new_owner` with no zero-address guard (unlike the shared library's `transfer_ownership` which does check).
- No contract emits a standardized `OwnershipTransferred` or `AdminChanged` event.
- No contract implements a two-step ownership transfer.
- `dex/factory` has no access control of any kind (see GEM-01-H04).

This fragmentation means each contract must be independently audited, monitoring tooling cannot rely on a consistent event schema, and a developer fixing one contract's pattern does not benefit the others.

**IMPACT:**  
Elevated per-contract risk. Zero-address ownership, single-step transfers, and missing event emissions are individually LOW findings per contract but collectively represent a HIGH systemic risk across the platform.

**AFFECTED:**  
All GEM contracts.

**FIX:**  
See GEM-01-H04 Fix option 1 (library integration). At minimum, standardize field naming to `owner: AccountId` across all contracts, add zero-address guards to all ownership/admin init and transfer functions, and add `OwnershipTransferred` events.

**CWE:** CWE-710 (Improper Adherence to Coding Standards)

---

### GEM-01-L11 — `RoleType = u8` Limits Role Namespace to 256 Values

```
SEVERITY       : LOW
LOCATION       : lib.rs line ~20 — type alias RoleType = u8
PATTERN        : RBAC
```

**DESCRIPTION:**  
`RoleType` is defined as `u8`, restricting the role namespace to 256 possible values (0–255). This is inconsistent with the EVM convention of `bytes32` roles (which are typically `keccak256` hashes of human-readable strings) and limits extensibility. Role constants 0–4 are already allocated (`DEFAULT_ADMIN_ROLE` through `UPGRADER_ROLE`), leaving 251 values for consuming contract roles — sufficient today but potentially limiting for complex permission models.

Using `u8` also means role identifiers are not self-documenting (unlike `keccak256("MINTER_ROLE")` bytes32 hashes), which increases the risk of collisions when multiple teams independently add roles.

**IMPACT:**  
Low immediate risk. Extensibility concern for complex multi-contract permission schemes.

**AFFECTED:**  
All consumers of `AccessControlData`.

**FIX:**  
Consider widening to `u32` or `[u8; 32]` before mainnet deployment. A migration from `u8` to a wider type after mainnet deployment will require storage migration and contract upgrade, making this a "fix before mainnet" concern.

**CWE:** CWE-190 (Integer Overflow or Wraparound — indirect risk from type width)

---

### GEM-01-L12 — `faucet::transfer_ownership` Has No Zero-Address Guard

```
SEVERITY       : LOW
LOCATION       : `faucet/lib.rs` line 162 — inline `transfer_ownership`
PATTERN        : Ownable (inline implementation)
```

**DESCRIPTION:**  
The faucet's inline `transfer_ownership` sets `self.owner = new_owner` without checking `new_owner != AccountId::default()`. The shared `OwnableData::transfer_ownership` (L134) correctly performs this check. Because the faucet does not use the shared library, it does not inherit this protection. A transaction passing `AccountId::default()` (32 zero bytes) as the new owner would succeed, transferring ownership to an unrecoverable burn address.

**IMPACT:**  
Accidental permanent loss of faucet ownership.

**AFFECTED:**  
`faucet` contract only.

**FIX:**

```rust
fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<(), FaucetError> {
    self.ensure_owner()?;
    if new_owner == AccountId::from([0u8; 32]) {
        return Err(FaucetError::ZeroAddress);
    }
    self.owner = new_owner;
    Ok(())
}
```

**CWE:** CWE-20 (Improper Input Validation)

---

### GEM-01-L13 — `simple_dao::transfer_admin` Has No Zero-Address Guard

```
SEVERITY       : LOW
LOCATION       : `simple_dao/lib.rs` line 358 — inline `transfer_admin`
PATTERN        : Ownable (inline implementation)
```

**DESCRIPTION:**  
Identical issue to GEM-01-L12. `simple_dao::transfer_admin` sets `self.admin = new_admin` without a zero-address check. Passing `AccountId::default()` transfers DAO administration to an irrecoverable burn address.

**IMPACT:**  
Accidental permanent loss of DAO admin control.

**AFFECTED:**  
`simple_dao` contract only.

**FIX:**  
Add `if new_admin == AccountId::from([0u8; 32]) { return Err(...) }` before the assignment.

**CWE:** CWE-20 (Improper Input Validation)

---

### GEM-01-I14 — Storage Layout Undocumented and Potentially Unstable

```
SEVERITY       : INFORMATIONAL
LOCATION       : access_control/lib.rs — all structs
PATTERN        : Storage
```

**DESCRIPTION:**  
`OwnableData`, `AccessControlData`, and `PausableData` have no documented storage key assignments. In ink! 5, manual storage keys can be specified via `#[ink(storage_key = ...)]` on `Mapping` fields. Without explicit keys, ink! derives them from the struct field names and positions. Any future rename or reordering of fields will change derived storage keys, silently corrupting all deployed contract state.

No ink! storage key derivation documentation exists in the codebase for these types.

**FIX:**  
Annotate all `Mapping` fields with explicit `#[ink(storage_key = ...)]` values. Document the current key assignments in a storage layout file. Add a changelog entry for any future storage changes with migration instructions.

---

### GEM-01-I15 — `OwnableData` and `AccessControlData` Are Not Cross-Referenced

```
SEVERITY       : INFORMATIONAL
LOCATION       : lib.rs — library-level design
PATTERN        : Ownable / RBAC
```

**DESCRIPTION:**  
The `OwnableData` and `AccessControlData` components are independent. A contract that uses both can reach a state where the owner is one account and the DEFAULT_ADMIN_ROLE holders are a different set of accounts. There is no mechanism for ensuring ownership and role-admin state remain consistent, and no documented guidance on which pattern takes precedence for which operations. For example, if ownership is renounced while the contract is paused and PAUSER_ROLE holders exist, there is no unified governance path.

**FIX:**  
Document the intended relationship between Ownable and RBAC in `docs/guides/BEST_PRACTICES.md`. Consider a `transfer_ownership` hook that also transfers DEFAULT_ADMIN_ROLE, or explicitly state that the two systems are orthogonal.

---

### GEM-01-I16 — ink! Version Discrepancy Between Code and Documentation

```
SEVERITY       : INFORMATIONAL
LOCATION       : access_control/Cargo.toml line ~5; audit brief
PATTERN        : Storage
```

**DESCRIPTION:**  
`access_control/Cargo.toml` pins `ink = "=5.1.1"`. The audit brief and several documentation files reference ink! 5.0. This discrepancy is minor but should be resolved to prevent confusion during dependency audits and supply-chain reviews.

**FIX:**  
Update all documentation references from "ink! 5.0" to "ink! 5.1.1". Confirm all other workspace crates pin the same ink! version to avoid version mismatches in the workspace build.

---

### GEM-01-I17 — Test Module Uses `unwrap()` Without Explanation

```
SEVERITY       : INFORMATIONAL
LOCATION       : lib.rs lines 598, 618, 624 — test module
PATTERN        : Storage
```

**DESCRIPTION:**  
Three `unwrap()` calls exist in the test module. These are in test code only and carry no production risk. They are noted because a future contributor copying test patterns into production code would introduce panic-on-error paths in access control functions, which the audit brief classifies as "Must Fix Before Next Audit Phase" for production code.

**FIX:**  
Replace with `expect("descriptive message")` or proper `assert_eq!` / `assert!(result.is_ok())` patterns to make test failures self-documenting.

---

## Library-Wide Impact Matrix

| Contract | Imports Ownable | Imports RBAC | Imports Pausable | Blast Radius if CRITICAL Found |
|---|---|---|---|---|
| `dalla_token` | **NO** — inline `owner: AccountId` | **NO** | **NO** | None from library; inline AC is separately unaudited |
| `beli_nft` | **NO** — inline `owner: AccountId` | **NO** | **NO** | None from library; inline AC is separately unaudited |
| `psp37_multi_token` | **NO** — inline `owner: AccountId` | **NO** | **NO** | None from library; inline AC is separately unaudited |
| `simple_dao` | **NO** — inline `admin: AccountId` | **NO** | **NO** | None from library; inline AC is separately unaudited |
| `dex/factory` | **NO** | **NO** | **NO** | None from library; **no access control at all** — independently HIGH risk |
| `dex/pair` | **NO** | **NO** | **NO** — custom reentrancy lock only | None from library; no pause mechanism present |
| `dex/router` | **NO** | **NO** | **NO** | None from library; deadline guard only |
| `faucet` | **NO** — inline `owner: AccountId` | **NO** | **NO** | None from library; inline AC has zero-address gap (GEM-01-L12) |

**All cells confirm: blast radius from a library vulnerability is zero today because the library is not integrated. This is itself the finding: the platform lacks the shared security baseline the library was built to provide.**

---

## Remediation Priority

| Priority | Finding | Blocker Type |
|---|---|---|
| P0 — Immediate | GEM-01-C01: `unpause` no access control | Hard Blocker |
| P0 — Immediate | GEM-01-C02: `pause` no access control | Hard Blocker |
| P0 — Immediate | GEM-01-C03: `renounce_role` no caller identity enforcement | Hard Blocker |
| P0 — Immediate | GEM-01-H07: single-step ownership transfer | Hard Blocker (audit criterion) |
| P1 — Before Integration | GEM-01-H04: library unused; `dex/factory` has no AC | Must Fix Before Mainnet |
| P1 — Before Integration | GEM-01-H05: last-admin revocation | Must Fix Before Integration |
| P1 — Before Integration | GEM-01-H06: `renounce_ownership` irrevocable, no confirmation | Must Fix Before Integration |
| P2 — Before Mainnet | GEM-01-M08: DEFAULT_ADMIN_ROLE hierarchy inversion | Must Fix Before Mainnet |
| P2 — Before Mainnet | GEM-01-M09: DEFAULT_ADMIN_ROLE unconstrained proliferation | Must Fix Before Mainnet |
| P2 — Before Mainnet | GEM-01-M10: siloed inconsistent access control patterns | Must Fix Before Mainnet |
| P3 — Low Risk | GEM-01-L11: `RoleType = u8` | Fix Before Mainnet (migration cost) |
| P3 — Low Risk | GEM-01-L12: `faucet` no zero-address guard on transfer | Fix Before Mainnet |
| P3 — Low Risk | GEM-01-L13: `simple_dao` no zero-address guard on transfer | Fix Before Mainnet |
| P4 — Informational | GEM-01-I14 through GEM-01-I17 | Best Effort |

---

## Auditor Notes

1. **The library's "passive" architecture is sound in isolation.** Accepting `caller: AccountId` as an explicit parameter rather than calling `self.env().caller()` internally is a valid and testable design. The failure is in the Pausable module's complete omission of even an authorization hook, and in `renounce_role`'s omission of a caller-identity enforcement precondition.

2. **The non-integration finding (GEM-01-H04) is the most significant systemic risk.** The library represents weeks of design work that is providing zero security benefit today. Every contract on the platform is operating on unaudited, inconsistent, one-off access control. Integrating the library (after fixing CRITICAL findings) should be the highest priority task.

3. **`dex/factory` requires immediate triage.** A DEX factory with no owner, no role, and no pause mechanism is a critical operational risk independent of this audit. The factory controls pair creation and potentially fee configurations that affect all trades on the platform.

4. **Two-step ownership transfer (GEM-01-H07) is a Hard Blocker per the audit's own pass criteria.** The code is functionally correct today, but the single-step pattern is a known source of permanent lockouts and is a non-negotiable requirement at the security standards this platform is targeting.

---

*End of AUDIT-GEM-01 — Access Control Library Security Audit*
