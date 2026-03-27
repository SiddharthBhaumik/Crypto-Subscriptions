#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, Symbol, Vec, Map,
};

// ─── Storage Keys ────────────────────────────────────────────────────────────

const PLANS: Symbol = symbol_short!("PLANS");
const SUBS: Symbol  = symbol_short!("SUBS");
const OWNER: Symbol = symbol_short!("OWNER");
const TOKEN: Symbol = symbol_short!("TOKEN");

// ─── Data Types ──────────────────────────────────────────────────────────────

/// A subscription plan created by the contract owner
#[contracttype]
#[derive(Clone, Debug)]
pub struct Plan {
    pub plan_id:       u64,
    pub name:          soroban_sdk::String,
    pub price:         i128,   // in stroops (1 XLM = 10_000_000)
    pub interval_days: u64,    // billing cycle length in days
    pub active:        bool,
}

/// A subscriber's current subscription state
#[contracttype]
#[derive(Clone, Debug)]
pub struct Subscription {
    pub subscriber:  Address,
    pub plan_id:     u64,
    pub start_ledger: u32,
    pub next_billing: u32,   // ledger sequence when next payment is due
    pub cancelled:   bool,
}

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {

    // ── Initialise ──────────────────────────────────────────────────────────

    /// Deploy the contract.
    /// `token_address` – the Soroban token (e.g. wrapped XLM) used for payments.
    pub fn initialize(env: Env, owner: Address, token_address: Address) {
        owner.require_auth();
        if env.storage().instance().has(&OWNER) {
            panic!("already initialised");
        }
        env.storage().instance().set(&OWNER, &owner);
        env.storage().instance().set(&TOKEN, &token_address);
        // Seed empty maps
        let plans: Map<u64, Plan>             = Map::new(&env);
        let subs:  Map<Address, Subscription> = Map::new(&env);
        env.storage().instance().set(&PLANS, &plans);
        env.storage().instance().set(&SUBS,  &subs);
    }

    // ── Owner: Plan Management ───────────────────────────────────────────────

    /// Create a new subscription plan (owner only).
    pub fn create_plan(
        env:          Env,
        name:         soroban_sdk::String,
        price:        i128,
        interval_days: u64,
    ) -> u64 {
        let owner: Address = env.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();

        let mut plans: Map<u64, Plan> = env.storage().instance().get(&PLANS).unwrap();
        let plan_id = plans.len() as u64;

        let plan = Plan {
            plan_id,
            name,
            price,
            interval_days,
            active: true,
        };
        plans.set(plan_id, plan);
        env.storage().instance().set(&PLANS, &plans);
        plan_id
    }

    /// Deactivate a plan so new subscribers cannot join (owner only).
    pub fn deactivate_plan(env: Env, plan_id: u64) {
        let owner: Address = env.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();

        let mut plans: Map<u64, Plan> = env.storage().instance().get(&PLANS).unwrap();
        let mut plan = plans.get(plan_id).expect("plan not found");
        plan.active = false;
        plans.set(plan_id, plan);
        env.storage().instance().set(&PLANS, &plans);
    }

    // ── Subscriber Actions ───────────────────────────────────────────────────

    /// Subscribe to a plan.
    /// The first payment is taken immediately from `subscriber`.
    pub fn subscribe(env: Env, subscriber: Address, plan_id: u64) {
        subscriber.require_auth();

        let plans: Map<u64, Plan> = env.storage().instance().get(&PLANS).unwrap();
        let plan = plans.get(plan_id).expect("plan not found");
        if !plan.active {
            panic!("plan is not active");
        }

        let mut subs: Map<Address, Subscription> = env.storage().instance().get(&SUBS).unwrap();
        if let Some(existing) = subs.get(subscriber.clone()) {
            if !existing.cancelled {
                panic!("already has an active subscription");
            }
        }

        // Charge first billing cycle
        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let token_client = token::Client::new(&env, &token_address);
        let contract_id  = env.current_contract_address();
        token_client.transfer(&subscriber, &contract_id, &plan.price);

        // Ledgers per day ≈ 17_280 (5-second average ledger time)
        let ledgers_per_day: u32 = 17_280;
        let current_ledger       = env.ledger().sequence();
        let next_billing         = current_ledger + (plan.interval_days as u32 * ledgers_per_day);

        let sub = Subscription {
            subscriber:   subscriber.clone(),
            plan_id,
            start_ledger: current_ledger,
            next_billing,
            cancelled: false,
        };
        subs.set(subscriber, sub);
        env.storage().instance().set(&SUBS, &subs);
    }

    /// Renew a subscription — callable by anyone once the billing date is reached.
    pub fn renew(env: Env, subscriber: Address) {
        let mut subs: Map<Address, Subscription> = env.storage().instance().get(&SUBS).unwrap();
        let mut sub = subs.get(subscriber.clone()).expect("subscription not found");

        if sub.cancelled {
            panic!("subscription is cancelled");
        }
        if env.ledger().sequence() < sub.next_billing {
            panic!("renewal not due yet");
        }

        let plans: Map<u64, Plan> = env.storage().instance().get(&PLANS).unwrap();
        let plan = plans.get(sub.plan_id).expect("plan not found");

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let token_client = token::Client::new(&env, &token_address);
        let contract_id  = env.current_contract_address();
        token_client.transfer(&subscriber, &contract_id, &plan.price);

        let ledgers_per_day: u32 = 17_280;
        sub.next_billing += plan.interval_days as u32 * ledgers_per_day;
        subs.set(subscriber, sub);
        env.storage().instance().set(&SUBS, &subs);
    }

    /// Cancel an active subscription.
    pub fn cancel(env: Env, subscriber: Address) {
        subscriber.require_auth();

        let mut subs: Map<Address, Subscription> = env.storage().instance().get(&SUBS).unwrap();
        let mut sub = subs.get(subscriber.clone()).expect("subscription not found");
        if sub.cancelled {
            panic!("already cancelled");
        }
        sub.cancelled = true;
        subs.set(subscriber, sub);
        env.storage().instance().set(&SUBS, &subs);
    }

    // ── Owner: Treasury ──────────────────────────────────────────────────────

    /// Withdraw accumulated subscription revenue (owner only).
    pub fn withdraw(env: Env, amount: i128) {
        let owner: Address = env.storage().instance().get(&OWNER).unwrap();
        owner.require_auth();

        let token_address: Address = env.storage().instance().get(&TOKEN).unwrap();
        let token_client = token::Client::new(&env, &token_address);
        let contract_id  = env.current_contract_address();
        token_client.transfer(&contract_id, &owner, &amount);
    }

    // ── Read-only Queries ────────────────────────────────────────────────────

    /// Fetch a plan by ID.
    pub fn get_plan(env: Env, plan_id: u64) -> Plan {
        let plans: Map<u64, Plan> = env.storage().instance().get(&PLANS).unwrap();
        plans.get(plan_id).expect("plan not found")
    }

    /// Fetch all plans.
    pub fn list_plans(env: Env) -> Vec<Plan> {
        let plans: Map<u64, Plan> = env.storage().instance().get(&PLANS).unwrap();
        plans.values()
    }

    /// Fetch a subscriber's current subscription.
    pub fn get_subscription(env: Env, subscriber: Address) -> Subscription {
        let subs: Map<Address, Subscription> = env.storage().instance().get(&SUBS).unwrap();
        subs.get(subscriber).expect("subscription not found")
    }

    /// Returns true if the subscriber has a non-cancelled, still-valid subscription.
    pub fn is_active(env: Env, subscriber: Address) -> bool {
        let subs: Map<Address, Subscription> = env.storage().instance().get(&SUBS).unwrap();
        match subs.get(subscriber) {
            Some(sub) => !sub.cancelled && env.ledger().sequence() <= sub.next_billing,
            None      => false,
        }
    }
}
