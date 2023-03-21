#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{ensure_eq, Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Uint128};
use cw2::set_contract_version;
use sg_std::Response;

use crate::error::ContractError;
use crate::helpers::{ensure_admin, ensure_not_paused};
use crate::msg::{ExecuteMsg, FlipExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::state::{CONFIG, FEES, FLIPS, TODO_FLIPS};
use crate::types::{Config, Wallets};

use crate::sudo;

// version info for migration info
const CONTRACT_NAME: &str = "cosmos-coin-flip";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimum amount of tokens we need to have in the contract
pub const MIN_BANK_AMOUNT: Uint128 = Uint128::new(30_000_000_000);

/// Min bet people are allow to bet
pub const MIN_BET: Uint128 = Uint128::new(5_000_000);
/// Max bet people are allow to bet
pub const MAX_BET: Uint128 = Uint128::new(25_000_000);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Verify the wallets are correct.
    deps.api.addr_validate(&msg.wallets.team)?;
    deps.api.addr_validate(&msg.wallets.reserve)?;

    let sg721_addr = match msg.sg721_addr {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };

    // Save config
    CONFIG.save(
        deps.storage,
        &Config {
            admin: info.sender.to_string(),
            denoms: msg.denoms,
            bank_limit: msg.bank_limit.unwrap_or(MIN_BANK_AMOUNT),
            flips_per_block_limit: msg.flips_per_block_limit.unwrap_or(10), // 10 flips per block
            wallets: Wallets {
                team: msg.wallets.team,
                reserve: msg.wallets.reserve,
            },
            fees: msg.fees,
            sg721_addr,
            is_paused: false,
        },
    )?;

    // Init fees to be 0
    FEES.save(deps.storage, &Uint128::zero())?;
    FLIPS.save(deps.storage, &vec![])?;
    TODO_FLIPS.save(deps.storage, &vec![])?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match msg {
        ExecuteMsg::Flip(FlipExecuteMsg::StartFlip { pick, amount }) => {
            ensure_not_paused(&config)?;
            flip_execute::execute_start_flip(deps, env, info, &config, pick, amount)
        }
        ExecuteMsg::Flip(FlipExecuteMsg::DoFlips {}) => {
            ensure_not_paused(&config)?;
            flip_execute::execute_do_flips(deps, env, &config)
        }
        ExecuteMsg::Sudo(SudoMsg::Distribute {}) => {
            ensure_admin(&config, &info)?;
            sudo::distribute(deps, env, &config)
        }
        ExecuteMsg::Sudo(SudoMsg::UpdateFees { fees }) => {
            ensure_admin(&config, &info)?;
            sudo::update_fees(deps, config, fees)
        }
        ExecuteMsg::Sudo(SudoMsg::UpdateBankLimit { limit }) => {
            ensure_admin(&config, &info)?;
            sudo::update_bank_limit(deps, config, limit)
        }
        ExecuteMsg::Sudo(SudoMsg::UpdateSg721 { addr }) => {
            ensure_admin(&config, &info)?;
            sudo::update_sg721(deps, config, addr)
        }
        ExecuteMsg::Sudo(SudoMsg::UpdatePause(is_paused)) => {
            ensure_admin(&config, &info)?;
            sudo::update_pause(deps, config, is_paused)
        }
    }
}

mod flip_execute {

    use cw_utils::must_pay;

    use cosmwasm_std::{coin, ensure, BankMsg, Event, Uint128};
    use sha256::Sha256Digest;

    use crate::helpers::ensure_correct_funds;
    use crate::state::{get_next_flip_id, FEES, FLIPS, FLIP_ID, SCORES};
    use crate::types::{Flip, FlipScore, PickTypes, TodoFlip};

    use super::*;

    pub(crate) fn execute_start_flip(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        config: &Config,
        pick: PickTypes,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        // Make sure that the sent amount is not above our max
        ensure!(
            amount <= MAX_BET,
            ContractError::OverTheLimitBet {
                max_limit: MAX_BET.checked_div(Uint128::new(1000000))?.to_string()
            }
        );
        ensure!(
            amount >= MIN_BET,
            ContractError::UnderTheLimitBet {
                min_limit: MIN_BET.checked_div(Uint128::new(1000000))?.to_string()
            }
        );

        let mut todo_flips = TODO_FLIPS.load(deps.storage)?;

        // Make sure the user doesn't have flip waiting already
        ensure!(
            todo_flips
                .clone()
                .into_iter()
                .all(|x| x.wallet != info.sender),
            ContractError::AlreadyStartedFlip
        );

        // Make sure we only have 10 waiting flips max
        ensure!(
            (todo_flips.len() as u64) < config.flips_per_block_limit,
            ContractError::BlockLimitReached
        );

        // Verify we only have one coin sent
        if info.funds.len() != 1 {
            return Err(ContractError::WrongFundsAmount);
        }

        let funds = info.funds[0].clone();
        // Verify the sent funds is in supported denom.
        let denom = if config.denoms.clone().into_iter().any(|x| *x == funds.denom) {
            funds.denom
        } else {
            return Err(ContractError::WrongDenom { denom: funds.denom });
        };

        // Make sure the paid amount is correct (funds sent is the amount + fee)
        let fee_amount = ensure_correct_funds(funds.amount, amount, config.fees.flip_bps)?;
        let should_pay_amount = amount.checked_add(fee_amount)?;
        let paid_amount = must_pay(&info, &denom)?;

        ensure_eq!(
            should_pay_amount,
            paid_amount,
            ContractError::WrongPaidAmount
        );

        // Make sure we have funds to pay for the flip
        let mut fees = FEES.load(deps.storage)?;
        let balance = deps
            .querier
            .query_balance(&env.contract.address, denom.clone())?;
        ensure!(
            balance.amount - fees >= amount * Uint128::new(2),
            ContractError::ContractMissingFunds
        );

        // Save fees
        fees = fees.checked_add(fee_amount)?;
        FEES.save(deps.storage, &fees)?;

        let id = get_next_flip_id(deps.storage);
        FLIP_ID.save(deps.storage, &id)?;

        // Everything is correct, save this to_do_flip
        todo_flips.push(TodoFlip {
            id,
            wallet: info.sender,
            amount: coin(amount.u128(), denom),
            pick,
            block: env.block.height,
            timestamp: env.block.time,
        });
        TODO_FLIPS.save(deps.storage, &todo_flips)?;

        Ok(Response::default()
            .add_event(Event::new("start_flip").add_attribute("id", id.to_string())))
    }

    pub(crate) fn execute_do_flips(
        deps: DepsMut,
        env: Env,
        config: &Config,
    ) -> Result<Response, ContractError> {
        let todo_flips = TODO_FLIPS.load(deps.storage)?;

        // Make sure we have funds to pay for all the flips
        let fees = FEES.load(deps.storage)?;
        let total_amount_to_pay = todo_flips
            .iter()
            .fold(Uint128::zero(), |acc, x| acc + x.amount.amount);
        let balance = deps
            .querier
            .query_balance(&env.contract.address, config.denoms[0].clone())?;
        ensure!(
            balance.amount - fees >= total_amount_to_pay * Uint128::new(2),
            ContractError::ContractMissingFunds
        );

        let mut msgs = vec![];
        let mut response = Response::default();
        let rand = get_random(&env);
        let mut last_flips = FLIPS.load(deps.storage)?;
        let save_todo_flips: Vec<TodoFlip> = todo_flips
            .into_iter()
            .filter_map(|todo_flip| {
                // Filter out the flips that are not ready to be flipped.
                if todo_flip.block >= env.block.height {
                    Some(todo_flip)
                } else {
                    // This flip is ready to be flipped.
                    let flip_result = do_a_flip(&todo_flip, rand);

                    // Handle score and save it (needed the streak info in Flip)
                    let score = match SCORES.load(deps.storage, &todo_flip.wallet) {
                        Ok(mut score) => score.update(flip_result, env.clone()),
                        Err(_) => FlipScore::new(flip_result, env.clone()),
                    };
                    SCORES
                        .save(deps.storage, &todo_flip.wallet, &score)
                        .unwrap();

                    // Create new flip and save it
                    let flip = Flip {
                        wallet: todo_flip.wallet.clone(),
                        amount: todo_flip.amount.clone(),
                        result: flip_result,
                        streak: score.streak,
                        timestamp: env.block.time,
                    };

                    // Update last flips vector
                    if last_flips.len() >= 5 {
                        last_flips.remove(0);
                    }
                    last_flips.push(flip);

                    // Send funds if they won
                    if flip_result {
                        let pay = todo_flip.amount.amount * Uint128::new(2); // double the amount
                        msgs.push(BankMsg::Send {
                            to_address: todo_flip.wallet.to_string(),
                            amount: vec![coin(pay.u128(), todo_flip.amount.denom.clone())],
                        });
                    }

                    response = response.clone().add_event(
                        Event::new("flip")
                            .add_attribute("flipper", todo_flip.wallet)
                            .add_attribute("flip_id", todo_flip.id.to_string())
                            .add_attribute("flip_amount", todo_flip.amount.to_string())
                            .add_attribute("flip_pick", format!("{:?}", todo_flip.pick))
                            .add_attribute("result", if flip_result { "won" } else { "lost" }),
                    );

                    None
                }
            })
            .collect();

        FLIPS.save(deps.storage, &last_flips).unwrap();
        TODO_FLIPS.save(deps.storage, &save_todo_flips)?;

        Ok(response
            .add_attribute("flip_action", "do_flips")
            .add_messages(msgs))
    }

    fn get_random(env: &Env) -> u64 {
        let tx_index = if let Some(tx) = &env.transaction {
            tx.index
        } else {
            0
        };

        let sha256 = Sha256Digest::digest(format!(
            "{}{}{}",
            env.block.height,
            env.block.time.nanos(),
            tx_index,
        ));

        sha256.as_bytes().iter().fold(0, |acc, x| acc + *x as u64)
    }

    fn do_a_flip(todo_flip: &TodoFlip, rand: u64) -> bool {
        let er = todo_flip
            .wallet
            .as_bytes()
            .iter()
            .fold(0, |acc, x| acc + *x as u64);

        let flip_result = (rand + er) % 2 == 0;

        // if picked heads and flip_result is true, he won
        let won_heads = todo_flip.pick == PickTypes::Heads && flip_result;
        // if picked tails and flip_result is false, he won
        let won_tails = todo_flip.pick == PickTypes::Tails && !flip_result;

        // Return true if one of them is true (won) else return false (lost)
        won_heads || won_tails
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetLast5 {} => query::get_last_5(deps),
        QueryMsg::GetFeesAmount {} => query::get_fees(deps),
        QueryMsg::GetScore { address } => query::get_score(deps, address),
        QueryMsg::GetConfig {} => query::get_config(deps),
        QueryMsg::ShouldDoFlips {} => query::should_do_flips(deps, env),
        QueryMsg::DryDistribution {} => query::dry_distribution(deps, env),
    }
}

mod query {
    use cosmwasm_std::{to_binary, Binary, Decimal, Deps, Env, StdError, StdResult, Uint128};

    use crate::{
        msg::DryDistributionResponse,
        state::{CONFIG, FEES, FLIPS, SCORES, TODO_FLIPS},
        sudo::{calculate_fees_to_pay, get_holders_list, verify_contract_balance},
        types::FeesToPay,
    };

    pub fn get_fees(deps: Deps) -> StdResult<Binary> {
        to_binary(&FEES.load(deps.storage)?)
    }

    pub fn get_config(deps: Deps) -> StdResult<Binary> {
        to_binary(&CONFIG.load(deps.storage)?)
    }

    pub fn get_score(deps: Deps, address: String) -> StdResult<Binary> {
        let address = deps.api.addr_validate(&address)?;
        to_binary(&SCORES.load(deps.storage, &address)?)
    }

    pub fn should_do_flips(deps: Deps, env: Env) -> StdResult<Binary> {
        let todo_flips = TODO_FLIPS.load(deps.storage)?;

        let res = todo_flips
            .iter()
            .any(|todo_flip| env.block.height > todo_flip.block);
        to_binary(&res)
    }

    pub fn get_last_5(deps: Deps) -> StdResult<Binary> {
        let flips = FLIPS.load(deps.storage)?;

        to_binary(&flips)
    }

    pub fn dry_distribution(deps: Deps, env: Env) -> StdResult<Binary> {
        let config = CONFIG.load(deps.storage)?;
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
        ) = calculate_fees_to_pay(&config, total_fees)
            .map_err(|x| StdError::generic_err(x.to_string()))?;

        let reserve_fees_to_send = verify_contract_balance(
            deps,
            env,
            denom,
            total_fees,
            reserve_fees,
            config.bank_limit,
        )
        .map_err(|x| StdError::generic_err(x.to_string()))?;

        let mut paid_to_holders = Uint128::zero();
        let mut total_shares = Decimal::zero();
        let mut fees_per_token = Decimal::zero();
        let mut number_of_holders: u64 = 0;

        if !holders_fees_to_send.is_zero() {
            let (calculated_total_shares, holders_list) = get_holders_list(deps, sg721_addr)
                .map_err(|x| StdError::generic_err(x.to_string()))?;
            total_shares = calculated_total_shares;
            number_of_holders = holders_list.len() as u64;

            fees_per_token = Decimal::from_atomics(holders_fees_to_send, 0)
                .map_err(|x| StdError::generic_err(x.to_string()))?
                .checked_div(total_shares)
                .map_err(|x| StdError::generic_err(x.to_string()))?;

            for (_, num) in holders_list {
                let amount = fees_per_token.checked_mul(num)?.to_uint_floor();

                if !amount.is_zero() {
                    paid_to_holders = paid_to_holders.checked_add(amount)?;
                }
            }
        }

        to_binary(&DryDistributionResponse {
            total_fees,
            team_total_fee: team_fees_to_send,
            reserve_total_fee: reserve_fees_to_send,
            holders_total_fee: holders_fees_to_send,
            holders_total_shares: total_shares,
            fees_per_token,
            pay_to_holders: paid_to_holders,
            number_of_holders,
        })
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
