use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

use crate::types::{Config, Fees, Flip, FlipScore, PickTypes, Wallets};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub denoms: Vec<String>,
    pub wallets: Wallets,
    pub fees: Fees,
    pub bank_limit: Option<Uint128>,
    pub flips_per_block_limit: Option<u64>,
    pub sg721_addr: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Flip msgs
    Flip(FlipExecuteMsg),
    /// Only call-able by admin (mutlisig)
    Sudo(SudoMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(Config)]
    GetConfig {},
    /// Get fees total
    #[returns(Uint128)]
    GetFeesAmount {},
    /// Get last 10 flips
    #[returns(Vec<Flip>)]
    GetLast5 {},
    /// Get score of wallet
    #[returns(FlipScore)]
    GetScore { address: String },
    /// let us know if we should execute the do flips msg or not
    /// this is to prevent sending unnecessary txs
    #[returns(bool)]
    ShouldDoFlips {},
    #[returns(DryDistributionResponse)]
    DryDistribution {},
}

#[cw_serde]
pub enum FlipExecuteMsg {
    StartFlip { pick: PickTypes, amount: Uint128 },
    DoFlips {},
}

#[cw_serde]
pub enum SudoMsg {
    Distribute {},
    UpdateFees { fees: Fees },
    UpdateSg721 { addr: String },
    UpdateBankLimit { limit: Uint128 },
    UpdatePause(bool),
}

#[cw_serde]
pub enum MigrateMsg {
    Basic {},
}

#[cw_serde]
pub struct DryDistributionResponse {
    pub total_fees: Uint128,
    pub team_total_fee: Uint128,
    pub reserve_total_fee: Uint128,
    pub holders_total_fee: Uint128,
    pub holders_total_shares: Decimal,
    pub fees_per_token: Decimal,
    pub pay_to_holders: Uint128,
    pub number_of_holders: u64,
}
