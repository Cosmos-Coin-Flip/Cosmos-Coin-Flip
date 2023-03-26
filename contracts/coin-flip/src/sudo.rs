use std::collections::HashMap;

use cosmwasm_std::{coins, Addr, BankMsg, Decimal, Deps, DepsMut, Env, Uint128};
use sg_std::Response;

use crate::error::ContractError;
use crate::state::{CONFIG, FEES};
use crate::types::{Config, Fees, FeesToPay};

/// Update the bank limit in the config
pub fn update_bank_limit(
    deps: DepsMut,
    mut config: Config,
    limit: Uint128,
) -> Result<Response, ContractError> {
    config.bank_limit = limit;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_bank_limit"))
}

pub fn update_fees(
    deps: DepsMut,
    mut config: Config,
    fees: Fees,
) -> Result<Response, ContractError> {
    config.fees = fees;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_fees"))
}

pub fn update_sg721(
    deps: DepsMut,
    mut config: Config,
    addr: String,
) -> Result<Response, ContractError> {
    config.sg721_addr = Some(deps.api.addr_validate(&addr)?);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_sg721"))
}

pub fn update_bet_limit(
    deps: DepsMut,
    mut config: Config,
    min_bet: Uint128,
    max_bet: Uint128,
) -> Result<Response, ContractError> {
    config.min_bet_limit = min_bet;
    config.max_bet_limit = max_bet;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_bet_limit"))
}

pub fn update_pause(
    deps: DepsMut,
    mut config: Config,
    is_paused: bool,
) -> Result<Response, ContractError> {
    config.is_paused = is_paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("method", "update_pause"))
}

pub fn distribute(deps: DepsMut, env: Env, config: &Config) -> Result<Response, ContractError> {
    let total_fees = FEES.load(deps.storage).unwrap_or_default();

    // TODO: Currently we only accept 1 denom
    let denom = config.denoms[0].clone();
    let (
        sg721_addr,
        FeesToPay {
            team: team_fees_to_send,
            holders: holders_fees_to_send,
            reserve: reserve_fees,
        },
    ) = calculate_fees_to_pay(config, total_fees)?;

    let reserve_fees_to_send = verify_contract_balance(
        deps.as_ref(),
        env,
        denom.clone(),
        total_fees,
        reserve_fees,
        config.bank_limit,
    )?;

    // Handle holders fees
    let mut msgs: Vec<BankMsg> = vec![];
    let mut paid_to_holders = Uint128::zero();
    let mut total_shares = Decimal::zero();
    let mut fees_per_token = Decimal::zero();

    if !holders_fees_to_send.is_zero() {
        let (calculated_total_shares, holders_list) = get_holders_list(deps.as_ref(), sg721_addr)?;
        total_shares = calculated_total_shares;

        fees_per_token =
            Decimal::from_atomics(holders_fees_to_send, 0)?.checked_div(total_shares)?;

        for (addr, num) in holders_list {
            let amount = fees_per_token.checked_mul(num)?.to_uint_floor();

            if !amount.is_zero() {
                msgs.push(BankMsg::Send {
                    to_address: addr,
                    amount: coins(amount.into(), denom.clone()),
                });
            }

            paid_to_holders = paid_to_holders.checked_add(amount)?;
        }
    }

    // create subMsg send to team wallet
    msgs.push(BankMsg::Send {
        to_address: config.wallets.team.clone(),
        amount: coins(team_fees_to_send.into(), denom.clone()),
    });

    // Send to reserve
    if !reserve_fees_to_send.is_zero() {
        msgs.push(BankMsg::Send {
            to_address: config.wallets.reserve.clone(),
            amount: coins(reserve_fees_to_send.into(), denom),
        });
    }

    // calculate remaining fees and save them to state
    let remaining_fees = total_fees
        .checked_sub(paid_to_holders)?
        .checked_sub(team_fees_to_send)?
        .checked_sub(reserve_fees)?;
    FEES.save(deps.storage, &remaining_fees)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("total_fees", total_fees)
        .add_attribute("reserve_paid", reserve_fees)
        .add_attribute("team_paid", team_fees_to_send)
        .add_attribute("holders_paid", paid_to_holders)
        .add_attribute("fees_per_token", fees_per_token.to_string())
        .add_attribute("total_shares", total_shares.to_string()))
}

pub fn calculate_fees_to_pay(
    config: &Config,
    total_fees: Uint128,
) -> Result<(Addr, FeesToPay), ContractError> {
    // If fees are lower then the minimum bet amount, means we don't fees to pay (no flips happened)
    if total_fees.u128() <= 1000_u128 {
        return Err(ContractError::NoFeesToPay {});
    }

    // If we have sg721_addr, it means we have a collection we need to distribute to
    // the holders. If not, we distribute to the team and reserve 50/50.
    if let Some(sg721_addr) = config.sg721_addr.clone() {
        Ok((sg721_addr, config.fees.calculate(total_fees)))
    } else {
        let half = total_fees.checked_div(Uint128::new(2))?;
        Ok((
            Addr::unchecked("sg721"),
            FeesToPay {
                team: half,
                holders: Uint128::zero(),
                reserve: half,
            },
        ))
    }
}

pub fn verify_contract_balance(
    deps: Deps,
    env: Env,
    denom: String,
    total_fees: Uint128,
    reserve_fees: Uint128,
    bank_limit: Uint128,
) -> Result<Uint128, ContractError> {
    let mut reserve_fees_to_send = reserve_fees;
    let contract_balance = deps.querier.query_balance(env.contract.address, denom)?;
    let bank_balance = contract_balance
        .amount
        .checked_sub(total_fees)
        .map_err(|_| ContractError::NotEnoughFundsToPayFees)?;

    if bank_balance < bank_limit {
        // How much we need to reach to the minimum bank amount.
        let reserve_diff = bank_limit.checked_sub(bank_balance)?;

        if reserve_diff > reserve_fees_to_send {
            // If we need more then we have, we send nothing.
            reserve_fees_to_send = Uint128::zero();
        } else {
            // If we need less then we have, we send the difference.
            reserve_fees_to_send.checked_sub(reserve_diff)?;
        }
    }
    Ok(reserve_fees_to_send)
}

pub fn get_holders_list(
    deps: Deps,
    sg721_addr: Addr,
) -> Result<(Decimal, HashMap<String, Decimal>), ContractError> {
    let mut total_shares = Decimal::zero();
    let mut holders_list: HashMap<String, Decimal> = HashMap::new();

    for num in 1..=777 {
        // Get the owner of the token.
        let owner_addr = deps.querier.query_wasm_smart::<cw721::OwnerOfResponse>(
            sg721_addr.clone(),
            &cw721::Cw721QueryMsg::OwnerOf {
                token_id: num.to_string(),
                include_expired: None,
            },
        );

        if let Ok(res) = owner_addr {
            let rewards_share = get_share(num)?;
            total_shares = total_shares.checked_add(rewards_share)?;

            holders_list
                .entry(res.owner)
                .and_modify(|e| *e += rewards_share)
                .or_insert(rewards_share);
        }
    }
    Ok((total_shares, holders_list))
}

pub fn get_share(num: u32) -> Result<Decimal, ContractError> {
    if (650..=727).contains(&num) {
        // 1.5
        Decimal::one()
            .checked_add(Decimal::from_atomics(Uint128::from(5_u128), 1)?)
            .map_err(ContractError::OverflowErr)
    } else if num >= 728 {
        Decimal::from_atomics(Uint128::from(2_u128), 0).map_err(ContractError::DecimalRangeExceeded)
    // 2
    } else {
        Ok(Decimal::one()) // 1
    }
}
