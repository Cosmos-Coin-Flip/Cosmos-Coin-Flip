use std::convert::Infallible;

use cosmwasm_std::{
    CheckedFromRatioError, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowErr(#[from] OverflowError),

    #[error("{0}")]
    Infallible(#[from] Infallible),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Fees to be paid is 0")]
    NoFeesToPay,

    #[error("Fees amount to distribute is more then the contract balance")]
    NotEnoughFundsToPayFees,

    #[error("The sent funds holds wrong amount.")]
    WrongPaidAmount,

    #[error("We do not support this denom = {denom}")]
    WrongDenom { denom: String },

    #[error("We only support 1 denom at a time.")]
    WrongFundsAmount,

    #[error("You already started a flip, please wait for it to finish.")]
    AlreadyStartedFlip,

    #[error("Block limit reached, please try again in few seconds")]
    BlockLimitReached,

    #[error("NFT contract is not set.")]
    Sg721NotSet,

    #[error("Contract doesn't have enough funds to pay for the bet")]
    ContractMissingFunds,

    #[error("You cannot bet above our limit = {max_limit}")]
    OverTheLimitBet { max_limit: String },

    #[error("You cannot bet under our limit = {min_limit}")]
    UnderTheLimitBet { min_limit: String },

    #[error("Operation is paused at this moment! Please try again later.")]
    Paused,
}
