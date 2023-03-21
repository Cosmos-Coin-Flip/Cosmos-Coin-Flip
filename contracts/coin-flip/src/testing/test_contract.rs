use cosmwasm_std::{coin, coins, Addr, Timestamp, Uint128};
use cw_multi_test::Executor;

use crate::{
    contract::{MAX_BET, MIN_BET},
    error::ContractError,
    msg::{ExecuteMsg, FlipExecuteMsg, SudoMsg},
    testing::utils::{executes::sudo_update_pause, helpers::MIN_FUNDS},
    types::{Fees, Flip, FlipScore, PickTypes, Streak},
};

use super::utils::{
    executes::{
        execute_do_flips, execute_start_flip, sudo_update_bank_limit, sudo_update_fees,
        unwrap_execute,
    },
    helpers::add_10_todo_flips,
    queries::{query_config, query_last_flips, query_score},
    setup::{setup_base_contract, FLIPPER_ADDR, FLIPPER_ADDR2, NATIVE_DENOM, PLUS_NANOS},
};

#[test]
fn test_do_single_flip() {
    let (mut app, contract_addr) = setup_base_contract();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    let flips: Vec<Flip> = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(
        flips[0],
        Flip {
            wallet: Addr::unchecked(FLIPPER_ADDR),
            amount: coin(MIN_BET.u128(), NATIVE_DENOM),
            result: false,
            streak: Streak {
                amount: 1,
                result: false
            },
            timestamp: Timestamp::from_seconds(123456789).plus_nanos(PLUS_NANOS),
        }
    );

    // lets match the score and make sure its correct.
    let score = query_score(&app, contract_addr, FLIPPER_ADDR.to_string()).unwrap();

    // because we set the block in setup, we know our flip is a lose on current block.
    assert_eq!(
        score,
        FlipScore {
            streak: Streak {
                amount: 1,
                result: false
            },
            last_flip: flips[0].timestamp
        }
    );
}

#[test]
fn test_multiple_flips() {
    let (mut app, contract_addr) = setup_base_contract();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR2),
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    let flips: Vec<Flip> = query_last_flips(&app, contract_addr.clone()).unwrap();

    assert!(flips.len() == 2);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr).unwrap();
}

/// only 1 active flip is allowed, so this should fail
#[test]
fn test_2_flips_in_a_row() {
    let (mut app, contract_addr) = setup_base_contract();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();

    let err = execute_start_flip(
        &mut app,
        contract_addr,
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::AlreadyStartedFlip);
}

#[test]
fn test_update_bank_limit() {
    let (mut app, contract_addr) = setup_base_contract();
    let new_limit = Uint128::new(150000000);

    sudo_update_bank_limit(&mut app, contract_addr.clone(), new_limit).unwrap();

    let config = query_config(&app, contract_addr).unwrap();

    assert_eq!(config.bank_limit, new_limit);
}

#[test]
fn test_update_config_fees() {
    let (mut app, contract_addr) = setup_base_contract();
    let new_fees = Fees {
        team_bps: 1000,
        holders_bps: 8000,
        reserve_bps: 1000,
        flip_bps: 400,
    };

    sudo_update_fees(&mut app, contract_addr.clone(), new_fees.clone()).unwrap();

    let config = query_config(&app, contract_addr).unwrap();

    assert_eq!(config.fees, new_fees);
}

#[test]
fn test_flips_per_block_limit() {
    let (mut app, contract_addr) = setup_base_contract();

    add_10_todo_flips(&mut app, contract_addr.clone());

    let err = execute_start_flip(
        &mut app,
        contract_addr,
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap_err();

    assert_eq!(err, ContractError::BlockLimitReached);
}

#[test]
fn test_wrong_funds() {
    let (mut app, contract_addr) = setup_base_contract();

    let err = execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS.checked_add(Uint128::one()).unwrap(),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::WrongPaidAmount);

    let err = unwrap_execute(app.execute_contract(
        Addr::unchecked(FLIPPER_ADDR),
        contract_addr.clone(),
        &ExecuteMsg::Flip(FlipExecuteMsg::StartFlip {
            pick: PickTypes::Heads,
            amount: MIN_BET,
        }),
        &[coin(MIN_FUNDS.u128(), NATIVE_DENOM), coin(1, "random")],
    ))
    .unwrap_err();

    assert_eq!(err, ContractError::WrongFundsAmount);

    let err = unwrap_execute(app.execute_contract(
        Addr::unchecked(FLIPPER_ADDR),
        contract_addr,
        &ExecuteMsg::Flip(FlipExecuteMsg::StartFlip {
            pick: PickTypes::Heads,
            amount: MIN_BET,
        }),
        &coins(MIN_FUNDS.u128(), "random"),
    ))
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::WrongDenom {
            denom: "random".to_string()
        }
    );
}

#[test]
fn test_sudo_unauth() {
    let (mut app, contract_addr) = setup_base_contract();

    let err = unwrap_execute(app.execute_contract(
        Addr::unchecked("random"),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateFees {
            fees: Fees {
                team_bps: 1500,
                holders_bps: 7000,
                reserve_bps: 1500,
                flip_bps: 300,
            },
        }),
        &[],
    ))
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized);
}

#[test]
fn test_max_min_amounts() {
    let (mut app, contract_addr) = setup_base_contract();

    let err = execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        Uint128::new(100000),
        Addr::unchecked(FLIPPER_ADDR),
        Uint128::new(103500),
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::UnderTheLimitBet {
            min_limit: MIN_BET
                .checked_div(Uint128::new(1000000))
                .unwrap()
                .to_string()
        }
    );

    let err = execute_start_flip(
        &mut app,
        contract_addr,
        PickTypes::Heads,
        MAX_BET.checked_add(Uint128::one()).unwrap(),
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::OverTheLimitBet {
            max_limit: MAX_BET
                .checked_div(Uint128::new(1000000))
                .unwrap()
                .to_string()
        }
    );
}

#[test]
fn test_no_flip_same_block() {
    let (mut app, contract_addr) = setup_base_contract();

    let flips = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(flips.len(), 0);

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();
    unwrap_execute(app.execute_contract(
        Addr::unchecked(FLIPPER_ADDR),
        contract_addr.clone(),
        &ExecuteMsg::Flip(FlipExecuteMsg::DoFlips {}),
        &[],
    ))
    .unwrap();

    let flips = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(flips.len(), 0);

    execute_do_flips(&mut app, contract_addr.clone()).unwrap();
    let flips = query_last_flips(&app, contract_addr).unwrap();
    assert_eq!(flips.len(), 1);
}

#[test]
fn test_missing_funds() {
    let (mut app, contract_addr) = setup_base_contract();

    add_10_todo_flips(&mut app, contract_addr.clone());

    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    app.execute(
        contract_addr.clone(),
        cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Burn {
            amount: coins(30000000000, NATIVE_DENOM),
        }),
    )
    .unwrap();

    let err = execute_start_flip(
        &mut app,
        contract_addr,
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap_err();
    assert_eq!(err, ContractError::ContractMissingFunds);
}

#[test]
fn test_missing_funds_on_do_funds() {
    let (mut app, contract_addr) = setup_base_contract();

    add_10_todo_flips(&mut app, contract_addr.clone());

    app.execute(
        contract_addr.clone(),
        cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Burn {
            amount: coins(30000000000, NATIVE_DENOM),
        }),
    )
    .unwrap();

    let err = execute_do_flips(&mut app, contract_addr).unwrap_err();
    assert_eq!(err, ContractError::ContractMissingFunds);
}

#[test]
fn test_contract_is_paused() {
    let (mut app, contract_addr) = setup_base_contract();

    sudo_update_pause(&mut app, contract_addr.clone(), true).unwrap();

    let err = execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR2),
        MIN_FUNDS,
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Paused);

    let err = execute_do_flips(&mut app, contract_addr.clone()).unwrap_err();
    assert_eq!(err, ContractError::Paused);

    // Make sure that the contract is unpaused
    sudo_update_pause(&mut app, contract_addr.clone(), false).unwrap();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Heads,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR2),
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr).unwrap();
}
