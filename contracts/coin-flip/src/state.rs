use cosmwasm_std::{Addr, StdError, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use crate::types::{Config, Flip, FlipScore, TodoFlip};

/// Our config holds admin and fees %
pub const CONFIG: Item<Config> = Item::new("config");
/// Fees that we collected since last distribution.
pub const FEES: Item<Uint128> = Item::new("total_fees");
/// Score per address, basically how much wins/loses/streaks, etc.
pub const SCORES: Map<&Addr, FlipScore> = Map::new("scores");
/// Last Flip id
pub const FLIP_ID: Item<u64> = Item::new("flip_id");
/// Flips tracker so we can easily get stats later
pub const FLIPS: Item<Vec<Flip>> = Item::new("last_flips");
pub const TODO_FLIPS: Item<Vec<TodoFlip>> = Item::new("todo_flips");

/// Get the current flip id
pub fn get_flip_id(store: &dyn Storage) -> Result<u64, StdError> {
    FLIP_ID.load(store)
}
/// helper function to get the next flip id.
pub fn get_next_flip_id(store: &dyn Storage) -> u64 {
    match get_flip_id(store) {
        Ok(res) => res + 1,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Uint128;

    use crate::types::{Fees, FeesToPay};

    #[test]
    fn test_fee_calc() {
        let fee = Fees {
            team_bps: 1500,
            holders_bps: 7000,
            reserve_bps: 1500,
            flip_bps: 350,
        };

        let total_fees = Uint128::new(100);

        let FeesToPay {
            team: team_fees_to_pay,
            holders: holders_fees_to_pay,
            reserve: reserve_fees_to_pay,
        } = fee.calculate(total_fees);

        assert_eq!(
            total_fees,
            team_fees_to_pay
                .checked_add(holders_fees_to_pay)
                .unwrap()
                .checked_add(reserve_fees_to_pay)
                .unwrap()
        );

        // Make sure floor is working correctly
        let total_fees = Uint128::new(150);

        let FeesToPay {
            team: team_fees_to_pay,
            holders: holders_fees_to_pay,
            reserve: reserve_fees_to_pay,
        } = fee.calculate(total_fees);

        assert_eq!(
            total_fees.checked_sub(Uint128::one()).unwrap(), // this is 149 (150 - 1)
            team_fees_to_pay // This should be 149 (22 + 22 + 105 = 149)
                .checked_add(holders_fees_to_pay)
                .unwrap()
                .checked_add(reserve_fees_to_pay)
                .unwrap()
        );

        println!(
            "{:?}",
            team_fees_to_pay
                .checked_add(holders_fees_to_pay)
                .unwrap()
                .checked_add(reserve_fees_to_pay)
                .unwrap()
        )
    }
}
