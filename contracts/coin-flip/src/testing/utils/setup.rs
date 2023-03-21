use cosmwasm_std::{
    coin, coins, testing::MockApi, Addr, BlockInfo, Empty, MemoryStorage, Timestamp,
};
use cw_multi_test::{
    App, BankKeeper, BasicAppBuilder, Contract, ContractWrapper, Executor, FailingModule,
    WasmKeeper,
};

use sg721::{CollectionInfo, RoyaltyInfoResponse};
use sg_std::StargazeMsgWrapper;

use crate::{
    msg::InstantiateMsg,
    types::{Fees, Wallets},
};

use super::{
    executes::sudo_update_sg721,
    helpers::{add_balance, mint_777_nfts},
};

pub type BaseApp = App<
    BankKeeper,
    MockApi,
    MemoryStorage,
    FailingModule<StargazeMsgWrapper, Empty, Empty>,
    WasmKeeper<StargazeMsgWrapper, Empty>,
>;

pub const FLIPPER_ADDR: &str = "some_flipper";
pub const FLIPPER_ADDR2: &str = "some_flipper2";
pub const CREATOR_ADDR: &str = "creator";
pub const NATIVE_DENOM: &str = "native_denom";

//Wallets
pub const TEAM_ADDR: &str = "team_wallet";
pub const RESERVE_ADDR: &str = "reserve_wallet";

pub const PLUS_NANOS: u64 = 654321;

pub fn flip_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn nft_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_nanos(PLUS_NANOS);
    block.height += 1;
}

/// Basic setup for unit test on a single contract
pub fn setup_base_contract() -> (BaseApp, Addr) {
    let mut app: BaseApp = BasicAppBuilder::<sg_std::StargazeMsgWrapper, Empty>::new_custom()
        .with_block(BlockInfo {
            height: 1,
            time: Timestamp::from_seconds(123456789),
            chain_id: "stargaze-1".to_string(),
        })
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(FLIPPER_ADDR),
                    vec![
                        coin(999999999999999, NATIVE_DENOM),
                        coin(999999999999999, "random"),
                    ],
                )
                .unwrap();

            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(FLIPPER_ADDR2),
                    coins(999999999999999, NATIVE_DENOM),
                )
                .unwrap();
        });

    let code_id = app.store_code(flip_contract());

    let init_msg = &InstantiateMsg {
        admin: CREATOR_ADDR.to_string(),
        denoms: vec![NATIVE_DENOM.to_string()],
        wallets: Wallets {
            team: TEAM_ADDR.to_string(),
            reserve: RESERVE_ADDR.to_string(),
        },
        fees: Fees {
            team_bps: 1500,
            holders_bps: 7000,
            reserve_bps: 1500,
            flip_bps: 350,
        },
        bank_limit: None,
        flips_per_block_limit: None,
        sg721_addr: None,
    };

    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR_ADDR),
            init_msg,
            &[],
            "flip contract",
            Some(CREATOR_ADDR.to_string()),
        )
        .unwrap();

    add_balance(&mut app, contract_addr.clone(), 30000000000);

    (app, contract_addr)
}

pub fn setup_contract() -> (BaseApp, Addr) {
    let (mut app, contract_addr) = setup_base_contract();
    let nft_code_id = app.store_code(nft_contract());

    let nft_addr = app
        .instantiate_contract(
            nft_code_id,
            contract_addr.clone(),
            &sg721::InstantiateMsg {
                name: "Test NFT".to_string(),
                symbol: "TEST".to_string(),
                minter: contract_addr.to_string(),
                collection_info: CollectionInfo::<RoyaltyInfoResponse> {
                    description: "Test NFT".to_string(),
                    image: "https://example.net".to_string(),
                    creator: CREATOR_ADDR.to_string(),
                    external_link: None,
                    explicit_content: Some(false),
                    start_trading_time: None,
                    royalty_info: None,
                },
            },
            &[],
            "flip contract",
            None,
        )
        .unwrap();

    mint_777_nfts(&mut app, nft_addr.clone(), contract_addr.clone());

    sudo_update_sg721(&mut app, contract_addr.clone(), nft_addr.to_string()).unwrap();

    (app, contract_addr)
}
