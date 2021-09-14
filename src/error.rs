use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Only accepts tokens in the cw20_whitelist")]
    NotInWhitelist {},

    #[error("Escrow is expired")]
    Expired {},

    #[error("Send some coins to create an escrow")]
    EmptyBalance {},

    #[error("Escrow id already in use")]
    AlreadyInUse {},

    #[error("Wager does not exist")]
    WagerDoesNotExist {},

    #[error("User does not exist")]
    UserDoesNotExist {},

    #[error("Balance sent does not equal to User 1 balance")]
    UnequalBalance {},

    #[error("Unknown contract error")]
    UnknownError {},
}
