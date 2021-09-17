#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg,
    WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Balance, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{config, config_read, GenericBalance, State, Wager, WAGERS};

// version info for migration info
const CONTRACT_NAME: &str = "duel-dojo:wager";
const CONTRACT_VERSION: &str = "0.1";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(deps: DepsMut, _env: Env, info: MessageInfo) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        creator: info.sender.clone(),
        owner: info.sender,
    };

    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        //DUEL DOJO FUNCTIONS
        ExecuteMsg::CreateWager { wager_id } => {
            execute_create_wager(deps, env, info.clone(), Balance::from(info.funds), wager_id)
        }
        ExecuteMsg::AddFunds { wager_id } => {
            execute_add_funds(deps, env, info.clone(), Balance::from(info.funds), wager_id)
        }
        ExecuteMsg::Cancel { wager_id } => execute_cancel(deps, env, info, wager_id),
        ExecuteMsg::SendFunds {
            wager_id,
            winner_address,
        } => execute_send_funds(deps, env, info, wager_id, winner_address),
    }
}

pub fn execute_create_wager(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    balance: Balance,
    wager_id: String,
) -> Result<Response, ContractError> {
    let user1_balance = match balance {
        Balance::Native(balance) => GenericBalance {
            native: balance.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => GenericBalance {
            native: vec![],
            cw20: vec![token],
        },
    };

    let state = config(deps.storage).load()?;

    //creates wager object
    let wager = Wager {
        arbiter: state.owner,
        user1: info.sender,
        user2: Addr::unchecked("empty"),
        user1_balance,
        user2_balance: GenericBalance {
            // initially empty
            native: vec![],
            cw20: vec![],
        },
    };

    // try to store it, fail if the id was already in use
    WAGERS.update(deps.storage, &wager_id, |existing| match existing {
        None => Ok(wager),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    })?;

    let res = Response::new().add_attributes(vec![("action", "create"), ("id", wager_id.as_str())]);
    Ok(res)
}

pub fn execute_add_funds(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    balance: Balance,
    wager_id: String,
) -> Result<Response, ContractError> {
    let mut wager = WAGERS.load(deps.storage, &wager_id).unwrap();

    //TODO: disallow user1 to equal to user2

    if wager.user2 != "empty" {
        return Err(ContractError::AlreadyInUse {});
    }

    let user2_balance = match balance.clone() {
        //TODO: rename
        Balance::Native(balance) => GenericBalance {
            native: balance.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => GenericBalance {
            native: vec![],
            cw20: vec![token],
        },
    };

    if user2_balance != wager.user1_balance {
        return Err(ContractError::UnequalBalance {});
    }

    wager.user2_balance.add_tokens(balance);
    wager.user2 = info.sender;

    WAGERS.update(deps.storage, &wager_id, |existing| match existing {
        None => Err(ContractError::WagerDoesNotExist {}),
        Some(_) => Ok(wager),
    })?;

    let res =
        Response::new().add_attributes(vec![("action", "add_funds"), ("id", wager_id.as_str())]);

    Ok(res)
}

pub fn execute_cancel(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    wager_id: String,
) -> Result<Response, ContractError> {
    let wager = WAGERS.load(deps.storage, &wager_id).unwrap();
    // let state = config(deps.storage).load()?;

    if info.sender == "" || info.sender != wager.user1 || wager.user2 != "empty" {
        Err(ContractError::Unauthorized {})
    } else {
        // we delete the wager
        WAGERS.remove(deps.storage, &wager_id);

        // send all tokens out
        let messages: Vec<SubMsg> = send_tokens(&wager.user1, &wager.user1_balance)?;

        Ok(Response::new()
            .add_attribute("action", "cancel")
            .add_attribute("id", wager_id)
            .add_attribute("to", wager.user1)
            .add_submessages(messages))
    }
}

pub fn execute_send_funds(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    wager_id: String,
    winner_address: Addr,
) -> Result<Response, ContractError> {
    let wager = WAGERS.load(deps.storage, &wager_id).unwrap();
    let state = config(deps.storage).load()?;

    if info.sender != state.owner {
        Err(ContractError::Unauthorized {})
    } else if winner_address != wager.user1 && winner_address != wager.user2 {
        Err(ContractError::UserDoesNotExist {})
    } else {
        // we delete the wager
        WAGERS.remove(deps.storage, &wager_id);

        // send user1 tokens to winner
        let user1_messages: Vec<SubMsg> = send_tokens(&winner_address, &wager.user1_balance)?;
        // send user2 tokens to winner
        let user2_messages: Vec<SubMsg> = send_tokens(&winner_address, &wager.user2_balance)?;

        Ok(Response::new()
            .add_attribute("action", "send_tokens_to_winner")
            .add_attribute("id", wager_id)
            .add_attribute("to", winner_address)
            .add_submessages(user1_messages)
            .add_submessages(user2_messages))
    }
}

fn send_tokens(to: &Addr, balance: &GenericBalance) -> StdResult<Vec<SubMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<SubMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.to_vec(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = SubMsg::new(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();
    msgs.append(&mut cw20_msgs?);
    Ok(msgs)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    //TODO: create query functions
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<State> {
    let state = config_read(deps.storage).load()?;
    Ok(state)
}

fn query_wager_for_id(id: String, deps: Deps) -> StdResult<Wager> {
    let wager = WAGERS.load(deps.storage, &id).unwrap();
    Ok(wager)
}

//TODO: change the names for different responses (into something different from "res")
//TODO: add assertions in all tests
#[cfg(test)]
mod tests {

    mod instantiate {
        use super::super::*;
        use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
        use cosmwasm_std::{attr, coins, CosmosMsg};

        #[test]
        fn test_initialization() {
            let info = mock_info("creator", &coins(0, "luna"));
            let mut deps = mock_dependencies(&[]);

            //check if the initialization works by unwrapping
            let res = instantiate(deps.as_mut(), mock_env(), info).unwrap();

            //check if state matches sender
            let res = query_config(deps.as_ref()).unwrap();
            assert_eq!("creator", res.creator.as_str());
            assert_eq!("creator", res.owner.as_str());
            print!("{}", res.owner.as_str());
        }
    }

    mod execute {
        use super::super::*;
        use crate::state::all_wager_ids;
        use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
        use cosmwasm_std::{attr, coins, CosmosMsg, Uint128};
        use cw20::{Cw20Coin, Cw20CoinVerified};

        #[test]
        fn test_execute_create_wager_native() {
            let info = mock_info("creator", &coins(0, "luna"));
            let mut deps = mock_dependencies(&[]);

            //initialize contract_addr
            let res = instantiate(deps.as_mut(), mock_env(), info.clone()).unwrap();

            let balance = Balance::from(coins(10, "uluna"));
            let wager_id = "test_id";

            let res = execute_create_wager(
                deps.as_mut(),
                mock_env(),
                info,
                balance,
                wager_id.parse().unwrap(),
            )
            .unwrap();
            let wagers = all_wager_ids(&deps.storage);
            let wager = query_wager_for_id("test_id".parse().unwrap(), deps.as_ref());

            // TODO add assertions
        }

        #[test]
        fn test_execute_create_wager_cw20() {
            let info = mock_info("creator", &coins(0, "luna"));
            let mut deps = mock_dependencies(&[]);

            //initialize contract_addr
            let res = instantiate(deps.as_mut(), mock_env(), info.clone()).unwrap();

            let coin = Cw20CoinVerified {
                address: Addr::unchecked("cw20-token"),
                amount: Uint128::new(100),
            };

            let new_user = mock_info("new_user", &coins(0, "luna"));

            let balance = Balance::from(coin);
            let wager_id = "test_id";

            let res = execute_create_wager(
                deps.as_mut(),
                mock_env(),
                new_user,
                balance,
                String::from(wager_id),
            )
            .unwrap();
            let wagers = all_wager_ids(&deps.storage);
            let wager = query_wager_for_id("test_id".parse().unwrap(), deps.as_ref());
            print!("{}", "");
            // TODO add assertions
        }

        #[test]
        fn test_execute_cancel_wager() {
            let info = mock_info("creator", &coins(0, "luna"));
            let mut deps = mock_dependencies(&[]);

            //initialize contract_addr
            let res = instantiate(deps.as_mut(), mock_env(), info.clone()).unwrap();

            let coin = Cw20CoinVerified {
                address: Addr::unchecked("cw20-token"),
                amount: Uint128::new(100),
            };

            let new_user = mock_info("new_user", &coins(0, "luna"));

            let balance = Balance::from(coin);
            let wager_id = "test_id";

            let res = execute_create_wager(
                deps.as_mut(),
                mock_env(),
                new_user.clone(),
                balance,
                String::from(wager_id),
            )
            .unwrap();

            let res = execute_cancel(deps.as_mut(), mock_env(), info, String::from(wager_id));

            let res2 = execute_cancel(
                deps.as_mut(),
                mock_env(),
                new_user.clone(),
                String::from(wager_id),
            )
            .unwrap();
            let wagers = all_wager_ids(&deps.storage).unwrap();

            print!("{}", "");
            // TODO add assertions
        }

        #[test]
        fn test_execute_add_funds() {
            let info = mock_info("creator", &coins(0, "luna"));
            let mut deps = mock_dependencies(&[]);

            //initialize contract_addr
            let res = instantiate(deps.as_mut(), mock_env(), info.clone()).unwrap();

            let coin = Cw20CoinVerified {
                address: Addr::unchecked("cw20-token"),
                amount: Uint128::new(100),
            };

            let new_user = mock_info("new_user", &coins(0, "luna"));

            let balance = Balance::from(coin);
            let wager_id = "test_id";

            let res = execute_create_wager(
                deps.as_mut(),
                mock_env(),
                new_user.clone(),
                balance,
                String::from(wager_id),
            )
            .unwrap();

            let coin2 = Cw20CoinVerified {
                address: Addr::unchecked("cw30-token"),
                amount: Uint128::new(99),
            };

            let balance2 = Balance::from(coin2);

            let new_user2 = mock_info("new_user", &coins(0, "luna"));

            let res2 = execute_add_funds(
                deps.as_mut(),
                mock_env(),
                new_user2.clone(),
                balance2,
                String::from(wager_id),
            );

            let wager = query_wager_for_id(String::from(wager_id), deps.as_ref());

            let coin3 = Cw20CoinVerified {
                address: Addr::unchecked("cw20-token"),
                amount: Uint128::new(100),
            };

            let balance3 = Balance::from(coin3);
            let res3 = execute_add_funds(
                deps.as_mut(),
                mock_env(),
                new_user2,
                balance3,
                String::from(wager_id),
            );
            print!("{}", "");
            // TODO add assertions
        }

        #[test]
        fn test_execute_send_funds() {
            let info = mock_info("creator", &coins(0, "luna"));
            let mut deps = mock_dependencies(&[]);

            //initialize contract_addr
            let res = instantiate(deps.as_mut(), mock_env(), info.clone()).unwrap();

            let coin = Cw20CoinVerified {
                address: Addr::unchecked("cw20-token"),
                amount: Uint128::new(100),
            };

            let new_user = mock_info("new_user", &coins(0, "luna"));

            let balance = Balance::from(coin);
            let wager_id = "test_id";

            let res = execute_create_wager(
                deps.as_mut(),
                mock_env(),
                new_user.clone(),
                balance,
                String::from(wager_id),
            )
            .unwrap();

            let coin2 = Cw20CoinVerified {
                address: Addr::unchecked("cw20-token"),
                amount: Uint128::new(100),
            };

            let balance2 = Balance::from(coin2);

            let new_user2 = mock_info("new_user2", &coins(0, "luna"));

            let res2 = execute_add_funds(
                deps.as_mut(),
                mock_env(),
                new_user2,
                balance2,
                String::from(wager_id),
            );

            let wager = query_wager_for_id("test_id".parse().unwrap(), deps.as_ref());

            let res3 = execute_send_funds(
                deps.as_mut(),
                mock_env(),
                new_user.clone(),
                String::from(wager_id),
                new_user.clone().sender,
            );

            let res4 = execute_send_funds(
                deps.as_mut(),
                mock_env(),
                info,
                String::from(wager_id),
                new_user.sender,
            );

            print!("{}", "");
            // TODO add assertions
        }
    }
}
