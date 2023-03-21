use cosmwasm_std::{ensure_eq, Decimal, MessageInfo, Uint128};

use crate::{error::ContractError, types::Config};

pub fn ensure_admin(config: &Config, info: &MessageInfo) -> Result<(), ContractError> {
    ensure_eq!(config.admin, info.sender, ContractError::Unauthorized);
    Ok(())
}

pub fn ensure_not_paused(config: &Config) -> Result<(), ContractError> {
    ensure_eq!(config.is_paused, false, ContractError::Paused);
    Ok(())
}

pub fn calc_flip_fee(amount: Decimal, fee: Decimal) -> Result<Uint128, ContractError> {
    let fee_to_pay = amount
        .checked_mul(fee)?
        .checked_div(Decimal::percent(100))?;
    // Decimal to Uint128
    let f = fee_to_pay
        .floor()
        .atomics()
        .checked_div(Uint128::from(10_u128).checked_pow(fee_to_pay.decimal_places())?)?;
    Ok(f)
}

/// Function to ensure flipper paid the right amount with fees
pub fn ensure_correct_funds(
    funds: Uint128,
    amount: Uint128,
    fee_bps: u64,
) -> Result<Uint128, ContractError> {
    let fee = bps_to_decimal(fee_bps);
    let fee_to_pay = calc_flip_fee(Decimal::from_atomics(amount, 0)?, fee)?;
    let total_amount = amount.checked_add(fee_to_pay).unwrap();
    if funds != total_amount {
        return Err(ContractError::WrongPaidAmount {});
    }
    Ok(fee_to_pay)
}

pub fn bps_to_decimal(bps: u64) -> Decimal {
    Decimal::percent(bps) / Uint128::from(100u128)
}

#[test]
fn test() {
    let res = ensure_correct_funds(Uint128::new(103_500), Uint128::new(100_000), 350).unwrap();
    assert_eq!(res, Uint128::new(3_500));
}
