#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    CreateMsg, DetailsResponse, ExecuteMsg, InstantiateMsg, ListResponse, QueryMsg,
};
use crate::state::{GenericBalance, Wager, OWNERS, WAGERS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-escrow";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let owner = msg.sender;
    OWNERS.update(deps.storage, &"contract_owner", |existing| match existing {
        None => Ok(owner),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    });

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
            execute_create_wager(deps, env, &info.sender, Balance::from(info.funds), wager_id)
        }
        ExecuteMsg::AddFunds { wager_id } => {
            execute_add_funds(deps, env, &info.sender, Balance::from(info.funds), wager_id)
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
    sender: &Addr,
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

    //creates wager object
    let wager = Wager {
        arbiter: deps.api.addr_validate(
            OWNERS
                .load(deps.storage, "contract_owner")
                .unwrap()
                .as_str(),
        )?,
        user1: sender.clone(),
        user2: Addr::unchecked("empty"),
        user1_balance: user1_balance,
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
    sender: &Addr,
    balance: Balance,
    wager_id: String,
) -> Result<Response, ContractError> {
    let mut wager = WAGERS.load(deps.storage, &wager_id).unwrap();

    if wager.user2 != "empty" {
        return Err(ContractError::AlreadyInUse {});
    }

    if balance != Balance::from(wager.user1_balance.cw20[0].clone()) {
        return Err(ContractError::UnequalBalance {});
    }

    wager.user2_balance.add_tokens(balance);
    wager.user2 = sender.clone();

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

    if info.sender != "" || info.sender != wager.user1 || wager.user2 != ""{
        return Err(ContractError::Unauthorized {});
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

    if info.sender
        != OWNERS
            .load(deps.storage, "contract_owner")
            .unwrap()
            .as_str()
    {
        return Err(ContractError::Unauthorized {});
    } else if winner_address != wager.user1 || winner_address != wager.user2 {
        return Err(ContractError::UserDoesNotExist {});
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

    }
}



