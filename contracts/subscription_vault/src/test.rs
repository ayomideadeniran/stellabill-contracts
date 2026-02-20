use crate::{Subscription, SubscriptionStatus, SubscriptionVault, SubscriptionVaultClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

#[test]
fn test_init_and_struct() {
    let env = Env::default();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    client.init(&token, &admin);
    // TODO: add create_subscription test with mock token
}

#[test]
fn test_subscription_struct() {
    let env = Env::default();
    let sub = Subscription {
        subscriber: Address::generate(&env),
        merchant: Address::generate(&env),
        amount: 10_000_0000,                 // 10 USDC (6 decimals)
        interval_seconds: 30 * 24 * 60 * 60, // 30 days
        last_payment_timestamp: 0,
        status: SubscriptionStatus::Active,
        prepaid_balance: 50_000_0000,
        usage_enabled: false,
    };
    assert_eq!(sub.status, SubscriptionStatus::Active);
}

#[test]
fn test_cancel_subscription_by_subscriber() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.init(&token, &admin);

    let sub_id = client.create_subscription(&subscriber, &merchant, &1000, &86400, &true);

    client.cancel_subscription(&sub_id, &subscriber);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);
}

#[test]
fn test_cancel_subscription_by_merchant() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.init(&token, &admin);

    let sub_id = client.create_subscription(&subscriber, &merchant, &1000, &86400, &true);

    client.cancel_subscription(&sub_id, &merchant);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);
}

#[test]
fn test_cancel_subscription_idempotence() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.init(&token, &admin);

    let sub_id = client.create_subscription(&subscriber, &merchant, &1000, &86400, &true);

    // Call once
    client.cancel_subscription(&sub_id, &subscriber);

    // Call again - should succeed idempotently without error
    client.cancel_subscription(&sub_id, &subscriber);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);
}

#[test]
#[should_panic(expected = "Error(Contract, #401)")]
fn test_cancel_subscription_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other = Address::generate(&env);

    client.init(&token, &admin);

    let sub_id = client.create_subscription(&subscriber, &merchant, &1000, &86400, &true);

    client.cancel_subscription(&sub_id, &other);
}

#[test]
fn test_withdraw_subscriber_funds() {
    let env = Env::default();
    env.mock_all_auths();

    // Setup mock token
    let admin = Address::generate(&env);
    let token_contract = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::Client::new(&env, &token_contract);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token_contract);

    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);

    let vault_admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.init(&token_contract, &vault_admin);

    // Mint some to the subscriber
    token_admin.mint(&subscriber, &5000);

    let sub_id = client.create_subscription(&subscriber, &merchant, &1000, &86400, &true);

    // Deposit funds to increase prepaid balance
    client.deposit_funds(&sub_id, &subscriber, &5000);

    // Cancel subscription
    client.cancel_subscription(&sub_id, &subscriber);

    // Withdraw funds
    client.withdraw_subscriber_funds(&sub_id, &subscriber);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.prepaid_balance, 0);
    assert_eq!(token.balance(&subscriber), 5000); // 5000 minted - 5000 deposited + 5000 withdrawn
    assert_eq!(token.balance(&contract_id), 0);
}
