use std::str::FromStr;

use cosmwasm_std::{coins, Addr, Decimal, Empty, Uint128};
use cosmwasm_storage::PrefixedStorage;
use cw_multi_test::{AppResponse, Executor};

use crate::{contract::MIN_BET, types::PickTypes};

use super::{
    executes::{execute_start_flip, unwrap_execute},
    setup::{BaseApp, NATIVE_DENOM},
};

pub const FLIPPER_PREFIX: &str = "flipper-";
// Might need change if min amount is changed
pub const MIN_FUNDS: Uint128 = Uint128::new(5175000);
pub const MIN_FEES: Uint128 = Uint128::new(175000);

pub fn update_storage(
    app: &mut BaseApp,
    address: &[u8],
    function: &mut dyn FnMut(&mut PrefixedStorage),
) {
    app.init_modules(|_, _, storage| {
        let mut namespace = b"contract_data/".to_vec();
        namespace.extend_from_slice(address);

        let mut prefixed_storage = PrefixedStorage::multilevel(storage, &[b"wasm", &namespace]);

        function(&mut prefixed_storage);
    })
}

pub fn add_balance(app: &mut BaseApp, addr: Addr, amount: u128) {
    app.init_modules(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &addr, coins(amount, NATIVE_DENOM))
            .unwrap();
    });
}

pub fn add_balances(app: &mut BaseApp, amount: u64) {
    app.init_modules(|router, _, storage| {
        for i in 0..amount {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(format!("{FLIPPER_PREFIX}{i}")),
                    coins(100000000, NATIVE_DENOM),
                )
                .unwrap();
        }
    });
}

pub fn add_10_todo_flips(app: &mut BaseApp, contract_addr: Addr) {
    add_balances(app, 10);

    for i in 0..10 {
        execute_start_flip(
            app,
            contract_addr.clone(),
            PickTypes::Heads,
            MIN_BET,
            Addr::unchecked(format!("{FLIPPER_PREFIX}{i}")),
            MIN_FUNDS,
        )
        .unwrap();
    }
}

pub fn mint_777_nfts(app: &mut BaseApp, nft_contract_addr: Addr, sender: Addr) {
    add_balances(app, 777);

    for i in 1..=777 {
        unwrap_execute(app.execute_contract(
            sender.clone(),
            nft_contract_addr.clone(),
            &sg721::ExecuteMsg::Mint::<Empty, Empty>(cw721_base::MintMsg {
                token_id: i.to_string(),
                owner: format!("{FLIPPER_PREFIX}{i}").to_string(),
                token_uri: None,
                extension: Empty {},
            }),
            &[],
        ))
        .unwrap();
    }
}

#[derive(Debug)]
pub struct DistResponse {
    pub total_fees: Uint128,
    pub reserve_paid: Uint128,
    pub team_paid: Uint128,
    pub holders_paid: Uint128,
    pub fees_per_token: Decimal,
    pub total_shares: Uint128,
}

pub fn get_dist_result(res: AppResponse) -> DistResponse {
    let event = res.events.into_iter().find(|e| e.ty == "wasm").unwrap();
    let mut total_fees = Uint128::zero();
    let mut reserve_paid = Uint128::zero();
    let mut team_paid = Uint128::zero();
    let mut holders_paid = Uint128::zero();
    let mut fees_per_token = Decimal::zero();
    let mut total_shares = Uint128::zero();

    event.attributes.into_iter().for_each(|attr| {
        if attr.key == "total_fees" {
            total_fees = Uint128::from_str(&attr.value).unwrap();
        } else if attr.key == "reserve_paid" {
            reserve_paid = Uint128::from_str(&attr.value).unwrap();
        } else if attr.key == "team_paid" {
            team_paid = Uint128::from_str(&attr.value).unwrap();
        } else if attr.key == "holders_paid" {
            holders_paid = Uint128::from_str(&attr.value).unwrap();
        } else if attr.key == "fees_per_token" {
            fees_per_token = Decimal::from_str(&attr.value).unwrap();
        } else if attr.key == "total_shares" {
            total_shares = Uint128::from_str(&attr.value).unwrap();
        }
    });

    DistResponse {
        total_fees,
        reserve_paid,
        team_paid,
        holders_paid,
        fees_per_token,
        total_shares,
    }
}
