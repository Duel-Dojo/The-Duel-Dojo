use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, Coin, StdResult};

use cw20::{Balance, Cw20Coin, Cw20ReceiveMsg};
use crate::state::GenericBalance;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub sender: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    //DUEL DOJO NEW FUNCTIONS
    // Creates an instance of the escrow and adds funds from User 1.
    // Creates an escrow ID that can later be referenced. Sets User 1
    // cancel permissions by adding wallet address to the escrow information bucket.
    CreateWager {
        wager_id: String,
    },

    // Adds funds into an existing escrow using an escrow ID.
    // Removes User 1 from cancel permissions by removing wallet
    // address from the escrow information bucket. At this point,
    // the escrow is not cancellable.
    AddFunds {
        wager_id: String,
    },

    // Cancels the match. Either User 1 or Game can execute this
    Cancel {
        wager_id: String,
    },

    // When winner is determined, the game sends out the wager
    // pot to the winning User. Only the Game can use this function.
    SendFunds {
        wager_id: String,
        winner_address: Addr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateMsg {
    /// id is a human-readable name for the escrow to use later
    /// 3-20 bytes of utf-8 text
    pub id: String,
    /// arbiter can decide to approve or refund the escrow
    pub arbiter: String,
    /// if approved, funds go to the recipient
    pub recipient: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_time: Option<u64>,
    /// Besides any possible tokens sent with the CreateMsg, this is a list of all cw20 token addresses
    /// that are accepted by the escrow during a top-up. This is required to avoid a DoS attack by topping-up
    /// with an invalid cw20 contract. See https://github.com/CosmWasm/cosmwasm-plus/issues/19
    pub cw20_whitelist: Option<Vec<String>>,
}

impl CreateMsg {
    pub fn addr_whitelist(&self, api: &dyn Api) -> StdResult<Vec<Addr>> {
        match self.cw20_whitelist.as_ref() {
            Some(v) => v.iter().map(|h| api.addr_validate(h)).collect(),
            None => Ok(vec![]),
        }
    }
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    //TODO: query messages
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListResponse {
    /// list all registered ids
    pub wagers: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DetailsResponse {
    /// id of this escrow
    pub id: String,
    /// arbiter can decide to approve or refund the escrow, this is typically the game address
    pub arbiter: Addr,
    /// creator of contract
    pub user1: Addr,
    /// Player 2 that joined the contract
    pub user2: Addr,
    /// Player 1 Balance in Native and Cw20 tokens
    /// When end height set and block height exceeds this value, the wager is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    // pub end_height: Option<u64>, // TODO: FIX END TIME AND END HEIGHT
    // FIX END TIME -> Wager expires 200 blocks AFTER wager is confirmed (20 mins, ~6 secs per block)
    // /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    // /// block time exceeds this value, the escrow is expired.
    // /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    // pub end_time: Option<u64>,
    /// Balance in Native and Cw20 tokens
    pub user1_balance: GenericBalance,
    /// Player 2 Balance in Native and Cw20 tokens
    pub user2_balance: GenericBalance,
    // /// All possible contracts that we accept tokens from
    // pub cw20_whitelist: Vec<Addr>, // TODO: WHITELIST?
    // WHITELIST? -> Only DUEL tokens available for wager
}
