#![cfg(test)]

use crate::{
    ContractError, SubscriptionContract, SubscriptionContractClient, SubscriptionCreated,
    SubscriptionRenewed, SECONDS_PER_DAY, SUBSCRIPTION_CREATED_EVENT_NAME,
    SUBSCRIPTION_RENEWED_EVENT_NAME,
};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, TryFromVal};

#[test]
fn test_subscribe_creates_subscription() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;
    let duration_days = 30u64;

    let expiry = client.subscribe(&wallet, &key_id, &duration_days);

    // Expiry should be current ledger + 30 days
    let expected_expiry = env.ledger().sequence() + (30 * SECONDS_PER_DAY);
    assert_eq!(expiry, expected_expiry);

    // Check subscription info
    let info = client.get_subscription(&wallet, &key_id);
    assert!(info.active);
    assert_eq!(info.expiry, expected_expiry);
    assert_eq!(info.days_remaining, 30);
}

#[test]
fn test_subscribe_with_zero_duration_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    let result = client.try_subscribe(&wallet, &key_id, &0);
    assert_eq!(result, Err(Ok(ContractError::NotPositiveDuration)));
}

#[test]
fn test_subscribe_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;
    let duration_days = 30u64;

    client.subscribe(&wallet, &key_id, &duration_days);

    // Verify event was emitted
    let mut matching = soroban_sdk::Vec::new(&env);
    for (_, topics, data) in env.events().all().iter() {
        if let Some(first) = topics.get(0) {
            if Symbol::try_from_val(&env, &first) == Ok(SUBSCRIPTION_CREATED_EVENT_NAME) {
                if let Ok(event) = SubscriptionCreated::try_from_val(&env, &data) {
                    matching.push_back(event);
                }
            }
        }
    }

    assert_eq!(matching.len(), 1);
    let event = matching.get(0).unwrap();
    assert_eq!(event.wallet, wallet);
    assert_eq!(event.key_id, key_id);
}

#[test]
fn test_renew_subscription_extends_expiry() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;
    let duration_days = 30u64;

    // Create initial subscription
    let initial_expiry = client.subscribe(&wallet, &key_id, &duration_days);

    // Renew subscription
    let new_expiry = client.renew_subscription(&wallet, &key_id, &duration_days);

    // New expiry should be old expiry + 30 days
    let expected_new_expiry = initial_expiry + (30 * SECONDS_PER_DAY);
    assert_eq!(new_expiry, expected_new_expiry);

    // Check subscription info
    let info = client.get_subscription(&wallet, &key_id);
    assert!(info.active);
    assert_eq!(info.expiry, expected_new_expiry);
    assert_eq!(info.days_remaining, 60);
}

#[test]
fn test_renew_subscription_with_zero_duration_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    // Create initial subscription
    client.subscribe(&wallet, &key_id, &30);

    // Try to renew with zero duration
    let result = client.try_renew_subscription(&wallet, &key_id, &0);
    assert_eq!(result, Err(Ok(ContractError::NotPositiveDuration)));
}

#[test]
fn test_renew_nonexistent_subscription_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    let result = client.try_renew_subscription(&wallet, &key_id, &30);
    assert_eq!(result, Err(Ok(ContractError::SubscriptionNotFound)));
}

#[test]
fn test_renew_subscription_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;
    let duration_days = 30u64;

    // Create initial subscription
    let initial_expiry = client.subscribe(&wallet, &key_id, &duration_days);

    // Renew subscription
    client.renew_subscription(&wallet, &key_id, &duration_days);

    // Verify renewal event was emitted
    let mut matching = soroban_sdk::Vec::new(&env);
    for (_, topics, data) in env.events().all().iter() {
        if let Some(first) = topics.get(0) {
            if Symbol::try_from_val(&env, &first) == Ok(SUBSCRIPTION_RENEWED_EVENT_NAME) {
                if let Ok(event) = SubscriptionRenewed::try_from_val(&env, &data) {
                    matching.push_back(event);
                }
            }
        }
    }

    assert_eq!(matching.len(), 1);
    let event = matching.get(0).unwrap();
    assert_eq!(event.wallet, wallet);
    assert_eq!(event.key_id, key_id);
    assert_eq!(event.old_expiry, initial_expiry);
}

#[test]
fn test_get_subscription_for_nonexistent_subscription() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    let info = client.get_subscription(&wallet, &key_id);
    assert!(!info.active);
    assert_eq!(info.expiry, 0);
    assert_eq!(info.days_remaining, 0);
}

#[test]
fn test_check_subscription_active() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    client.subscribe(&wallet, &key_id, &30);

    assert!(client.check_subscription(&wallet, &key_id));
}

#[test]
fn test_check_subscription_nonexistent() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    assert!(!client.check_subscription(&wallet, &key_id));
}

#[test]
fn test_check_subscription_expired() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    // Create subscription with 1 day duration
    client.subscribe(&wallet, &key_id, &1);

    // Advance ledger by 2 days
    env.ledger().set(env.ledger().sequence() + (2 * SECONDS_PER_DAY));

    // Subscription should now be expired
    assert!(!client.check_subscription(&wallet, &key_id));

    let info = client.get_subscription(&wallet, &key_id);
    assert!(!info.active);
    assert_eq!(info.days_remaining, 0);
}

#[test]
fn test_multiple_subscriptions_for_different_keys() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id_1 = 1u64;
    let key_id_2 = 2u64;

    client.subscribe(&wallet, &key_id_1, &30);
    client.subscribe(&wallet, &key_id_2, &60);

    assert!(client.check_subscription(&wallet, &key_id_1));
    assert!(client.check_subscription(&wallet, &key_id_2));

    let info_1 = client.get_subscription(&wallet, &key_id_1);
    let info_2 = client.get_subscription(&wallet, &key_id_2);

    assert_eq!(info_1.days_remaining, 30);
    assert_eq!(info_2.days_remaining, 60);
}

#[test]
fn test_days_remaining_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &contract_id);

    let wallet = Address::generate(&env);
    let key_id = 1u64;

    client.subscribe(&wallet, &key_id, &10);

    // Initially should have 10 days
    let info = client.get_subscription(&wallet, &key_id);
    assert_eq!(info.days_remaining, 10);

    // Advance by 3 days
    env.ledger().set(env.ledger().sequence() + (3 * SECONDS_PER_DAY));

    // Should now have 7 days remaining
    let info = client.get_subscription(&wallet, &key_id);
    assert_eq!(info.days_remaining, 7);
}
