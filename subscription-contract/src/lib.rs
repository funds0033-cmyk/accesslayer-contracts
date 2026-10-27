#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

/// Event name emitted when a subscription is created.
pub const SUBSCRIPTION_CREATED_EVENT_NAME: Symbol = symbol_short!("sub_crt");

/// Event name emitted when a subscription is renewed.
pub const SUBSCRIPTION_RENEWED_EVENT_NAME: Symbol = symbol_short!("sub_ren");

/// Event name emitted when a subscription expires.
pub const SUBSCRIPTION_EXPIRED_EVENT_NAME: Symbol = symbol_short!("sub_exp");

/// Seconds in a day.
pub const SECONDS_PER_DAY: u64 = 86_400;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    NotPositiveDuration = 1,
    SubscriptionNotFound = 2,
    ArithmeticOverflow = 3,
}

/// Subscription data storing expiry timestamp.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Subscription {
    pub expiry: u64,
}

/// Event data payload emitted when a subscription is created.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct SubscriptionCreated {
    pub wallet: Address,
    pub key_id: u64,
    pub expiry: u64,
}

/// Event data payload emitted when a subscription is renewed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct SubscriptionRenewed {
    pub wallet: Address,
    pub key_id: u64,
    pub old_expiry: u64,
    pub new_expiry: u64,
}

/// Event data payload emitted when a subscription expires.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct SubscriptionExpired {
    pub wallet: Address,
    pub key_id: u64,
    pub expiry: u64,
}

/// Information about a subscription including active status and days remaining.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct SubscriptionInfo {
    pub active: bool,
    pub expiry: u64,
    pub days_remaining: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Subscription(Address, u64),
}

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    /// Creates a new subscription for a wallet to access a creator key's gated content.
    /// Locks subscription fee (in a real implementation, this would handle payment).
    /// Sets expiry timestamp based on current ledger time + duration_days.
    pub fn subscribe(env: Env, wallet: Address, key_id: u64, duration_days: u64) -> Result<u64, ContractError> {
        if duration_days == 0 {
            return Err(ContractError::NotPositiveDuration);
        }

        let current_ledger = env.ledger().sequence();
        let duration_seconds = duration_days
            .checked_mul(SECONDS_PER_DAY)
            .ok_or(ContractError::ArithmeticOverflow)?;
        let expiry = current_ledger
            .checked_add(duration_seconds)
            .ok_or(ContractError::ArithmeticOverflow)?;

        let subscription = Subscription { expiry };
        let key = DataKey::Subscription(wallet.clone(), key_id);
        env.storage().persistent().set(&key, &subscription);

        env.events().publish(
            (SUBSCRIPTION_CREATED_EVENT_NAME, wallet.clone(), key_id),
            SubscriptionCreated {
                wallet,
                key_id,
                expiry,
            },
        );

        Ok(expiry)
    }

    /// Renews an existing subscription, extending the expiry by the same duration
    /// from the current expiry (not from current time).
    pub fn renew_subscription(env: Env, wallet: Address, key_id: u64, duration_days: u64) -> Result<u64, ContractError> {
        if duration_days == 0 {
            return Err(ContractError::NotPositiveDuration);
        }

        let key = DataKey::Subscription(wallet.clone(), key_id);
        let subscription: Subscription = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::SubscriptionNotFound)?;

        let old_expiry = subscription.expiry;
        let duration_seconds = duration_days
            .checked_mul(SECONDS_PER_DAY)
            .ok_or(ContractError::ArithmeticOverflow)?;
        let new_expiry = old_expiry
            .checked_add(duration_seconds)
            .ok_or(ContractError::ArithmeticOverflow)?;

        let updated_subscription = Subscription { expiry: new_expiry };
        env.storage().persistent().set(&key, &updated_subscription);

        env.events().publish(
            (SUBSCRIPTION_RENEWED_EVENT_NAME, wallet.clone(), key_id),
            SubscriptionRenewed {
                wallet,
                key_id,
                old_expiry,
                new_expiry,
            },
        );

        Ok(new_expiry)
    }

    /// Returns subscription information including active status, expiry, and days remaining.
    pub fn get_subscription(env: Env, wallet: Address, key_id: u64) -> SubscriptionInfo {
        let key = DataKey::Subscription(wallet.clone(), key_id);
        let current_ledger = env.ledger().sequence();

        if let Some(subscription) = env.storage().persistent().get::<_, Subscription>(&key) {
            let active = subscription.expiry > current_ledger;
            let days_remaining = if active {
                let remaining_seconds = subscription.expiry - current_ledger;
                remaining_seconds / SECONDS_PER_DAY
            } else {
                0
            };

            SubscriptionInfo {
                active,
                expiry: subscription.expiry,
                days_remaining,
            }
        } else {
            SubscriptionInfo {
                active: false,
                expiry: 0,
                days_remaining: 0,
            }
        }
    }

    /// Checks if a wallet has an active subscription to a creator key.
    /// Returns false if subscription doesn't exist or has expired.
    /// Intended to be called by gated content contracts for access control.
    pub fn check_subscription(env: Env, wallet: Address, key_id: u64) -> bool {
        let info = Self::get_subscription(env, wallet, key_id);
        info.active
    }
}

#[cfg(test)]
mod test;
