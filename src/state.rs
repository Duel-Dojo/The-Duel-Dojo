use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Env, HumanAddr, Order, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;

use cosmwasm_storage::{singleton_read, ReadonlySingleton, Singleton};
use cw20::{Balance, Cw20CoinVerified};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}

impl GenericBalance {
    pub fn add_tokens(&mut self, add: Balance) {
        match add {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount += token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount += token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }
}



// DUEL DOJO CODE BELOW TODO: remove above
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Wager {
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

impl Wager {
    // pub fn is_expired(&self, env: &Env) -> bool {
    //     if let Some(end_height) = self.end_height {
    //         if env.block.height > end_height {
    //             return true;
    //         }
    //     }
    //
    //     if let Some(end_time) = self.end_time {
    //         if env.block.time > Timestamp::from_seconds(end_time) {
    //             return true;
    //         }
    //     }
    //
    //     false
    // }
    //
    // pub fn human_whitelist(&self) -> Vec<String> {
    //     self.cw20_whitelist.iter().map(|a| a.to_string()).collect()
    // } // TODO: uncomment if we implement end time and human whitelist
}

pub const OWNERS: Map<&str, Addr> = Map::new("owner");

pub const WAGERS: Map<&str, Wager> = Map::new("wager");

/// This returns the list of ids for all registered escrows
pub fn all_wager_ids(storage: &dyn Storage) -> StdResult<Vec<String>> {
    WAGERS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).map_err(|_| StdError::invalid_utf8("parsing escrow key")))
        .collect()
}