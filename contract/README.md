# 🔄 CryptoSubscriptions — Soroban Smart Contract

> Decentralised, trustless subscription billing on the Stellar network.  
> No middlemen. No chargebacks. Just code.

---

## 📖 Project Description

**CryptoSubscriptions** is a smart contract written in Rust for the [Soroban](https://soroban.stellar.org/) platform on Stellar. It replaces the traditional payment-processor subscription model (Stripe, Braintree, etc.) with on-chain logic: subscription plans live in contract storage, payments flow directly through the Soroban token interface, and every billing event is verifiable on-chain.

The contract is intentionally minimal and composable — it can be integrated into any dApp that needs recurring revenue: SaaS products, DAOs distributing premium content, NFT membership tiers, API access gates, and more.

---

## ⚙️ What It Does

| Actor | Action | Description |
|-------|--------|-------------|
| **Owner** | `initialize` | Deploy and configure the contract with a payment token |
| **Owner** | `create_plan` | Define a plan with a name, price (in stroops), and billing interval |
| **Owner** | `deactivate_plan` | Stop new sign-ups for a plan without disrupting existing subscribers |
| **Owner** | `withdraw` | Pull accumulated revenue from the contract treasury |
| **Subscriber** | `subscribe` | Join a plan — first payment is taken immediately |
| **Subscriber** | `cancel` | Cancel at any time; access persists until the current period ends |
| **Anyone** | `renew` | Trigger the next billing cycle once the due ledger is reached |
| **Anyone** | `is_active` | Check whether an address holds a valid, unexpired subscription |
| **Anyone** | `get_plan` / `list_plans` | Read plan details |
| **Anyone** | `get_subscription` | Read a subscriber's full state |

Payments are denominated in any **Soroban-compatible token** (e.g. wrapped XLM or a custom stablecoin), set at deploy time.

---

## ✨ Features

### 🗂️ Flexible Plan Management
Create unlimited subscription tiers — monthly, weekly, annual, or any custom interval measured in ledgers. Plans can be deactivated at any time without affecting active subscribers.

### 💸 Instant On-Chain Payments
The first payment is collected atomically at subscription time using the Soroban token interface (`transfer`). No escrow, no delays — funds land in the contract treasury immediately.

### 🔁 Permissionless Renewal
Renewals can be triggered by anyone (a keeper bot, the dApp backend, or the subscriber themselves) once the `next_billing` ledger is reached. This keeps billing predictable without requiring a centralised cron job.

### 🔒 Auth-Gated Actions
Every state-changing action is protected by Stellar's native `require_auth()`. Subscribers can only manage their own subscriptions; only the owner can create plans or withdraw revenue.

### 📡 Composable Status Checks
`is_active(subscriber)` returns a single boolean — making it trivial for other contracts or off-chain services to gate access to premium features.

### 🏦 Treasury Withdrawals
Accumulated payments sit in the contract address until the owner calls `withdraw(amount)`, giving full control over cash-flow timing.

### 📦 Zero External Dependencies
The contract depends only on `soroban-sdk`. No oracles, no bridges, no third-party protocols.

---

## 🗂️ Project Structure

```
crypto-subscriptions/
├── Cargo.toml                          # Workspace manifest
├── README.md
└── contracts/
    └── subscription/
        ├── Cargo.toml                  # Contract crate manifest
        └── src/
            └── lib.rs                  # Contract source
```

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install-stellar-cli)

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked stellar-cli --features opt
```

### Build

```bash
stellar contract build
```

The compiled `.wasm` file is output to `target/wasm32-unknown-unknown/release/subscription.wasm`.

### Deployed Contract

The contract is live on **Stellar Testnet**:

| Field | Value |
|-------|-------|
| **Contract ID** | `CDSOI62WEURCXQDQXT57Z6UK56VAF6W6KVSIX4LXHH5CGYP32GN5ADA2` |
| **Network** | Testnet |
| **Explorer** | [View on Stellar Expert](https://stellar.expert/explorer/testnet/contract/CDSOI62WEURCXQDQXT57Z6UK56VAF6W6KVSIX4LXHH5CGYP32GN5ADA2) |

### Deploy Your Own Instance

```bash
# Fund a test account
stellar keys generate owner --network testnet --fund

# Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/subscription.wasm \
  --network testnet \
  --source owner
```

### Initialise

```bash
stellar contract invoke \
  --id CDSOI62WEURCXQDQXT57Z6UK56VAF6W6KVSIX4LXHH5CGYP32GN5ADA2 \
  --network testnet \
  --source owner \
  -- initialize \
  --owner <OWNER_ADDRESS> \
  --token_address <TOKEN_ADDRESS>
```

### Create a Plan (10 XLM / 30 days)

```bash
stellar contract invoke \
  --id CDSOI62WEURCXQDQXT57Z6UK56VAF6W6KVSIX4LXHH5CGYP32GN5ADA2 \
  --network testnet \
  --source owner \
  -- create_plan \
  --name "Pro Monthly" \
  --price 100000000 \
  --interval_days 30
```

### Subscribe

```bash
stellar contract invoke \
  --id CDSOI62WEURCXQDQXT57Z6UK56VAF6W6KVSIX4LXHH5CGYP32GN5ADA2 \
  --network testnet \
  --source alice \
  -- subscribe \
  --subscriber <ALICE_ADDRESS> \
  --plan_id 0
```

---

## 🔐 Security Considerations

- All privileged functions (`create_plan`, `deactivate_plan`, `withdraw`) are guarded by `require_auth()` against the owner address set at initialisation.
- Subscriber actions (`subscribe`, `cancel`) are guarded by `require_auth()` against the subscriber's own address — preventing third-party cancellations.
- The contract cannot be re-initialised once deployed.
- Token transfers use the Soroban token interface directly; the contract never holds private keys.

---

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

---

> Built with ❤️ on [Stellar Soroban](https://soroban.stellar.org/)