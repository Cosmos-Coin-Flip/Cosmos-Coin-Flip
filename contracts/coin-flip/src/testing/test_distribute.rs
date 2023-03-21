use cosmwasm_std::{coin, coins, Decimal, Uint128};
use cw_multi_test::Executor;

use crate::{
    error::ContractError,
    msg::DryDistributionResponse,
    state::FEES,
    testing::utils::{
        helpers::{get_dist_result, update_storage, MIN_FEES},
        queries::query_dry_distribution,
        setup::setup_base_contract,
    },
};

use super::utils::{
    executes::{execute_do_flips, sudo_distribute},
    helpers::{add_10_todo_flips, add_balance},
    queries::query_fees,
    setup::{setup_contract, NATIVE_DENOM, RESERVE_ADDR, TEAM_ADDR},
};

#[test]
fn test_distribute() {
    let (mut app, contract_addr) = setup_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    add_10_todo_flips(&mut app, contract_addr.clone());

    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    let total_fee_amount_to_pay = query_fees(&app, contract_addr.clone()).unwrap();
    // We did 10 flips, so fee should be 17500000
    assert_eq!(
        total_fee_amount_to_pay,
        MIN_FEES.checked_mul(Uint128::new(10)).unwrap()
    );

    //do dry distribute query
    let dry_dist = query_dry_distribution(&app, contract_addr.clone()).unwrap();
    assert_eq!(
        dry_dist,
        DryDistributionResponse {
            total_fees: MIN_FEES.checked_mul(Uint128::new(10)).unwrap(),
            team_total_fee: Uint128::new(262500),
            reserve_total_fee: Uint128::new(262500),
            holders_total_fee: Uint128::new(1225000),
            holders_total_shares: Decimal::from_atomics(Uint128::new(866), 0).unwrap(),
            fees_per_token: dry_dist.fees_per_token, // TODO: calculate the actual fee per token
            pay_to_holders: dry_dist.pay_to_holders, // This is calculation based on how much each holder has
            number_of_holders: 777
        }
    );

    // With current set up (10 flips), here is how much should be distributed.
    let res = sudo_distribute(&mut app, contract_addr.clone()).unwrap();
    let res_data = get_dist_result(res);

    let total_fee_amount_left = query_fees(&app, contract_addr.clone()).unwrap();
    // We distributed all fees except rounding
    assert_eq!(
        total_fee_amount_left,
        total_fee_amount_to_pay
            .checked_sub(res_data.holders_paid)
            .unwrap()
            .checked_sub(res_data.reserve_paid)
            .unwrap()
            .checked_sub(res_data.team_paid)
            .unwrap()
    );

    // lets check balances
    let contract_balance = app
        .wrap()
        .query_balance(contract_addr, NATIVE_DENOM)
        .unwrap();
    // Balance break-down:
    // 30000000000 (bank) + rounding
    assert_eq!(
        contract_balance,
        coin(
            Uint128::from(30000000000_u128)
                .checked_add(total_fee_amount_left)
                .unwrap()
                .into(),
            NATIVE_DENOM
        )
    );

    // this flipper won
    let random_flipper_balance = app.wrap().query_balance("flipper-1", NATIVE_DENOM).unwrap();
    assert!(random_flipper_balance.amount > Uint128::new(100000000));
    // This flipper lost
    let random_flipper_balance = app.wrap().query_balance("flipper-2", NATIVE_DENOM).unwrap();
    assert!(random_flipper_balance.amount < Uint128::new(100000000));

    // Make sure team balance is correct
    let team_balance = app.wrap().query_balance(TEAM_ADDR, NATIVE_DENOM).unwrap();
    assert_eq!(team_balance, coin(res_data.team_paid.into(), NATIVE_DENOM));

    let reserve_balance = app
        .wrap()
        .query_balance(RESERVE_ADDR, NATIVE_DENOM)
        .unwrap();
    assert_eq!(
        reserve_balance,
        coin(res_data.reserve_paid.into(), NATIVE_DENOM)
    );
}

#[test]
fn test_distribute_without_collection() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 30000000000);

    add_10_todo_flips(&mut app, contract_addr.clone());

    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    // Update fees to have uneven number (for rounding tests)
    update_storage(&mut app, contract_addr.as_bytes(), &mut |storage| {
        let fees = FEES.load(storage).unwrap();
        FEES.save(storage, &fees.checked_add(Uint128::new(1)).unwrap())
            .unwrap();
    });

    let total_fee_amount_to_pay = query_fees(&app, contract_addr.clone()).unwrap();
    // We did 10 flips of minimum bet, so fee should be 17500001
    assert_eq!(
        total_fee_amount_to_pay,
        MIN_FEES
            .checked_mul(Uint128::new(10))
            .unwrap()
            .checked_add(Uint128::one())
            .unwrap()
    );

    let res = sudo_distribute(&mut app, contract_addr.clone()).unwrap();
    let res_data = get_dist_result(res);

    // There should be 1 left because of rounding
    let total_fee_amount = query_fees(&app, contract_addr.clone()).unwrap();
    assert_eq!(
        total_fee_amount,
        total_fee_amount_to_pay
            .checked_sub(res_data.team_paid)
            .unwrap()
            .checked_sub(res_data.reserve_paid)
            .unwrap()
    );

    // Make sure team balance is correct
    let team_balance = app.wrap().query_balance(TEAM_ADDR, NATIVE_DENOM).unwrap();
    assert_eq!(team_balance, coin(res_data.team_paid.into(), NATIVE_DENOM));

    let reserve_balance = app
        .wrap()
        .query_balance(RESERVE_ADDR, NATIVE_DENOM)
        .unwrap();
    assert_eq!(
        reserve_balance,
        coin(res_data.reserve_paid.into(), NATIVE_DENOM)
    );
}

#[test]
fn test_failing_distribute() {
    let (mut app, contract_addr) = setup_contract();

    // try distribute without any flips done (no fees)
    let err = sudo_distribute(&mut app, contract_addr.clone()).unwrap_err();
    assert_eq!(err, ContractError::NoFeesToPay);

    add_10_todo_flips(&mut app, contract_addr.clone());
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    // Reduce the balance a little to test reserve functionality
    app.execute(
        contract_addr.clone(),
        cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Burn {
            amount: coins(20000000000, NATIVE_DENOM),
        }),
    )
    .unwrap();

    let res = sudo_distribute(&mut app, contract_addr.clone()).unwrap();
    let res_data = get_dist_result(res);

    // Make sure team balance is correct
    let team_balance = app.wrap().query_balance(TEAM_ADDR, NATIVE_DENOM).unwrap();
    assert_eq!(team_balance, coin(res_data.team_paid.into(), NATIVE_DENOM));

    // Reserve should be 0, because it kept in the contract for bank
    let reserve_balance = app
        .wrap()
        .query_balance(RESERVE_ADDR, NATIVE_DENOM)
        .unwrap();
    assert_eq!(reserve_balance, coin(0, NATIVE_DENOM));

    // remove funds from contract to make sure we don't try to distribute when no fees are available
    add_10_todo_flips(&mut app, contract_addr.clone());
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    app.execute(
        contract_addr.clone(),
        cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Burn {
            amount: coins(10002000000, NATIVE_DENOM),
        }),
    )
    .unwrap();

    let err = sudo_distribute(&mut app, contract_addr).unwrap_err();
    assert_eq!(err, ContractError::NotEnoughFundsToPayFees);
}
