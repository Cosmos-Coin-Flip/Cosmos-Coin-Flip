use cosmwasm_std::{coins, Addr, Uint128};
use cw_multi_test::{AppResponse, Executor};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, FlipExecuteMsg, SudoMsg},
    types::{Fees, PickTypes},
};

use super::setup::{next_block, BaseApp, CREATOR_ADDR, FLIPPER_ADDR, NATIVE_DENOM};

pub(crate) fn unwrap_execute(
    res: Result<AppResponse, anyhow::Error>,
) -> Result<AppResponse, ContractError> {
    match res {
        Ok(res) => Ok(res),
        Err(e) => Err(e.downcast().unwrap()),
    }
}

pub fn execute_start_flip(
    app: &mut BaseApp,
    contract_addr: Addr,
    pick: PickTypes,
    flip_amount: Uint128,
    flipper: Addr,
    funds: Uint128,
) -> Result<AppResponse, ContractError> {
    let funds = coins(funds.u128(), NATIVE_DENOM);
    unwrap_execute(app.execute_contract(
        flipper,
        contract_addr,
        &ExecuteMsg::Flip(FlipExecuteMsg::StartFlip {
            pick,
            amount: flip_amount,
        }),
        &funds,
    ))
}

pub fn execute_do_flips(
    app: &mut BaseApp,
    contract_addr: Addr,
) -> Result<AppResponse, ContractError> {
    app.update_block(next_block);
    unwrap_execute(app.execute_contract(
        Addr::unchecked(FLIPPER_ADDR),
        contract_addr,
        &ExecuteMsg::Flip(FlipExecuteMsg::DoFlips {}),
        &[],
    ))
}

pub fn sudo_update_fees(
    app: &mut BaseApp,
    contract_addr: Addr,
    fees: Fees,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateFees { fees }),
        &[],
    ))
}

pub fn sudo_update_bet_limit(
    app: &mut BaseApp,
    contract_addr: Addr,
    min_bet: Uint128,
    max_bet: Uint128,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateBetLimit { min_bet, max_bet }),
        &[],
    ))
}

pub fn sudo_update_pause(
    app: &mut BaseApp,
    contract_addr: Addr,
    pause: bool,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdatePause(pause)),
        &[],
    ))
}

pub fn sudo_update_sg721(
    app: &mut BaseApp,
    contract_addr: Addr,
    addr: String,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateSg721 { addr }),
        &[],
    ))
}

pub fn sudo_update_bank_limit(
    app: &mut BaseApp,
    contract_addr: Addr,
    limit: Uint128,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::UpdateBankLimit { limit }),
        &[],
    ))
}

pub fn sudo_distribute(
    app: &mut BaseApp,
    contract_addr: Addr,
) -> Result<AppResponse, ContractError> {
    unwrap_execute(app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        contract_addr,
        &ExecuteMsg::Sudo(SudoMsg::Distribute {}),
        &[],
    ))
}
