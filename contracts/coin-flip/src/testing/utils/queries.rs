use cosmwasm_std::{Addr, StdError, Uint128};

use crate::{
    msg::{DryDistributionResponse, QueryMsg},
    types::{Config, Flip, FlipScore},
};

use super::setup::BaseApp;

pub fn query_config(app: &BaseApp, contract_addr: Addr) -> Result<Config, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetConfig {})
}

pub fn query_fees(app: &BaseApp, contract_addr: Addr) -> Result<Uint128, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetFeesAmount {})
}

pub fn query_last_flips(app: &BaseApp, contract_addr: Addr) -> Result<Vec<Flip>, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetLast5 {})
}

pub fn query_score(
    app: &BaseApp,
    contract_addr: Addr,
    address: String,
) -> Result<FlipScore, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::GetScore { address })
}

pub fn query_dry_distribution(
    app: &BaseApp,
    contract_addr: Addr,
) -> Result<DryDistributionResponse, StdError> {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::DryDistribution {})
}
