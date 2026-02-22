//! Single charge logic (no auth). Used by charge_subscription and batch_charge.
//!
//! **PRs that only change how one subscription is charged should edit this file only.**

use crate::queries::get_subscription;
use crate::state_machine::validate_status_transition;
use crate::types::{Error, SubscriptionStatus};
use soroban_sdk::Env;

pub fn charge_one(env: &Env, subscription_id: u32) -> Result<(), Error> {
    let mut sub = get_subscription(env, subscription_id)?;

    if sub.status != SubscriptionStatus::Active && sub.status != SubscriptionStatus::GracePeriod {
        return Err(Error::NotActive);
    }

    let now = env.ledger().timestamp();
    let next_allowed = sub
        .last_payment_timestamp
        .checked_add(sub.interval_seconds)
        .ok_or(Error::Overflow)?;
    if now < next_allowed {
        return Err(Error::IntervalNotElapsed);
    }

    if sub.prepaid_balance < sub.amount {
        let grace_duration = crate::admin::get_grace_period(env).unwrap_or(0);
        let grace_expires = next_allowed
            .checked_add(grace_duration)
            .ok_or(Error::Overflow)?;

        if now < grace_expires {
            if sub.status != SubscriptionStatus::GracePeriod {
                validate_status_transition(&sub.status, &SubscriptionStatus::GracePeriod)?;
                sub.status = SubscriptionStatus::GracePeriod;
                env.storage().instance().set(&subscription_id, &sub);
            }
            return Err(Error::InsufficientBalance);
        } else {
            validate_status_transition(&sub.status, &SubscriptionStatus::InsufficientBalance)?;
            sub.status = SubscriptionStatus::InsufficientBalance;
            env.storage().instance().set(&subscription_id, &sub);
            return Err(Error::InsufficientBalance);
        }
    }

    sub.prepaid_balance = sub
        .prepaid_balance
        .checked_sub(sub.amount)
        .ok_or(Error::Overflow)?;
    sub.last_payment_timestamp = now;

    if sub.status == SubscriptionStatus::GracePeriod {
        validate_status_transition(&sub.status, &SubscriptionStatus::Active)?;
        sub.status = SubscriptionStatus::Active;
    }

    env.storage().instance().set(&subscription_id, &sub);
    Ok(())
}
