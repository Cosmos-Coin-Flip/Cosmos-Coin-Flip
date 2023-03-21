use cosmwasm_std::Addr;

use crate::{contract::MIN_BET, types::PickTypes};

use super::utils::{
    executes::{execute_do_flips, execute_start_flip},
    helpers::{add_balance, add_balances, FLIPPER_PREFIX, MIN_FEES, MIN_FUNDS},
    queries::{query_fees, query_last_flips},
    setup::{setup_base_contract, FLIPPER_ADDR, FLIPPER_ADDR2},
};

#[test]
fn test_get_fees() {
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

    let fees = query_fees(&app, contract_addr).unwrap();

    assert_eq!(fees, MIN_FEES)
}

#[test]
fn test_get_last_flips() {
    let (mut app, contract_addr) = setup_base_contract();
    add_balance(&mut app, contract_addr.clone(), 300000000);

    // Make sure its working if no flips yet.
    let flips = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(flips.len(), 0);

    // Make sure its working if only 1 flip
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

    let flips = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(flips.len(), 1);

    // Make sure its working for exactly 5.
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

    let flips = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(flips.len(), 5);

    // Make sure its working for 6 (more then 5)
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR2),
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    let flips = query_last_flips(&app, contract_addr.clone()).unwrap();
    assert_eq!(flips.len(), 5);
    assert_eq!(flips[0].wallet.to_string(), FLIPPER_ADDR.to_string());

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR2),
        MIN_FUNDS,
    )
    .unwrap();

    let flips = query_last_flips(&app, contract_addr).unwrap();
    assert_eq!(flips.len(), 5);
}

#[test]
fn test_randomness() {
    let (mut app, contract_addr) = setup_base_contract();
    let addrs = 100;
    add_balance(&mut app, contract_addr.clone(), 30000000000);
    add_balances(&mut app, addrs);

    let mut win = 0;
    let mut lose = 0;

    for i in 0..addrs {
        execute_start_flip(
            &mut app,
            contract_addr.clone(),
            PickTypes::Heads,
            MIN_BET,
            Addr::unchecked(format!("{FLIPPER_PREFIX}{i}")),
            MIN_FUNDS,
        )
        .unwrap();
        execute_do_flips(&mut app, contract_addr.clone()).unwrap();
        let flip = query_last_flips(&app, contract_addr.clone()).unwrap()[0].clone();

        if flip.result {
            win += 1;
        } else {
            lose += 1;
        }
    }

    println!("Win: {win}, Lose: {lose}");
}

#[test]
fn test_get_flip_todo() {
    let (mut app, contract_addr) = setup_base_contract();
    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();
    execute_do_flips(&mut app, contract_addr.clone()).unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR),
        MIN_FUNDS,
    )
    .unwrap();

    execute_start_flip(
        &mut app,
        contract_addr.clone(),
        PickTypes::Tails,
        MIN_BET,
        Addr::unchecked(FLIPPER_ADDR2),
        MIN_FUNDS,
    )
    .unwrap();

    let flips = query_last_flips(&app, contract_addr).unwrap();
    assert_eq!(flips.len(), 1);
}
