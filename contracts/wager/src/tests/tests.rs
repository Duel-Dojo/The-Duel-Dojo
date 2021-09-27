use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{GenericBalance, State, Wager};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, coins, from_binary, BankMsg, CosmosMsg, SubMsg};

#[test]
fn test_initialization() {
    let info = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: info.clone().sender,
    };

    //check if the initialization works by unwrapping
    let initialization_check = instantiate(deps.as_mut(), mock_env(), info, inst_msg).unwrap();
    assert_eq!(0, initialization_check.messages.len());

    //check if state matches sender
    let res_query_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: State = from_binary(&res_query_config).unwrap();

    assert_eq!("creator", config.creator.as_str());
}

#[test]
fn test_execute_create_wager_native() {
    let info = mock_info("creator", &coins(0, "luna"));
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: info.clone().sender,
    };

    let _initialization_check = instantiate(deps.as_mut(), mock_env(), info, inst_msg).unwrap();

    let wager_id = String::from("test_id");
    let new_user = mock_info("new_user", &coins(10, "uluna"));

    let _res_create_wager = execute(
        deps.as_mut(),
        mock_env(),
        new_user,
        ExecuteMsg::CreateWager {
            wager_id: wager_id.clone(),
        },
    )
    .unwrap();

    let res_query_wager =
        query(deps.as_ref(), mock_env(), QueryMsg::Wager { id: wager_id }).unwrap();

    let wager: Wager = from_binary(&res_query_wager).unwrap();
    assert_eq!("creator", wager.arbiter);
    assert_eq!("new_user", wager.user1);
    assert_eq!("empty", wager.user2);

    let test_user1_balance = GenericBalance {
        native: coins(10, "uluna"),
        cw20: vec![],
    };

    assert_eq!(test_user1_balance, wager.user1_balance);
    assert_eq!(GenericBalance::new(), wager.user2_balance);
}

#[test]
fn test_execute_cancel_wager() {
    let info = mock_info("creator", &coins(0, "luna"));
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: info.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check = instantiate(deps.as_mut(), mock_env(), info, inst_msg).unwrap();

    let new_user = mock_info("new_user", &coins(10, "uluna"));

    let wager_id = String::from("test_id");

    let _res_create_wager = execute(
        deps.as_mut(),
        mock_env(),
        new_user.clone(),
        ExecuteMsg::CreateWager {
            wager_id: wager_id.clone(),
        },
    )
    .unwrap();

    let sneaky_user = mock_info("sneaky_user", &vec![]);

    let _res_cancel_fail = execute(
        deps.as_mut(),
        mock_env(),
        sneaky_user,
        ExecuteMsg::Cancel {
            wager_id: wager_id.clone(),
        },
    );
    assert!(_res_cancel_fail.is_err());
    let _res_cancel_fail = execute(
        deps.as_mut(),
        mock_env(),
        new_user,
        ExecuteMsg::Cancel {
            wager_id: wager_id.clone(),
        },
    );

    // wager doesn't exist.
    assert!(query(deps.as_ref(), mock_env(), QueryMsg::Wager { id: wager_id }).is_err());
}

#[test]
fn test_execute_send_funds_native() {
    let info = mock_info("creator", &vec![]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: info.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check =
        instantiate(deps.as_mut(), mock_env(), info.clone(), inst_msg).unwrap();

    let new_user = mock_info("new_user", &coins(10, "uluna"));

    let wager_id = String::from("test_id");

    let _res_create_wager = execute(
        deps.as_mut(),
        mock_env(),
        new_user.clone(),
        ExecuteMsg::CreateWager {
            wager_id: wager_id.clone(),
        },
    )
    .unwrap();

    let new_user2 = mock_info("new_user2", &coins(10, "uluna"));

    let _res_add_funds = execute(
        deps.as_mut(),
        mock_env(),
        new_user2,
        ExecuteMsg::AddFunds {
            wager_id: wager_id.clone(),
        },
    );

    let res_send_funds_fail = execute(
        deps.as_mut(),
        mock_env(),
        new_user.clone(),
        ExecuteMsg::SendFunds {
            wager_id: wager_id.clone(),
            winner_address: new_user.sender.clone(),
        },
    );

    assert!(res_send_funds_fail.is_err());

    let res_send_funds_success = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::SendFunds {
            wager_id: wager_id.clone(),
            winner_address: new_user.sender.clone(),
        },
    )
    .unwrap();

    // Wager was cancelled.
    assert!(query(deps.as_ref(), mock_env(), QueryMsg::Wager { id: wager_id }).is_err());

    assert_eq!(
        res_send_funds_success.messages[0],
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: String::from(new_user.sender.clone()),
            amount: vec![coin(10, "uluna")],
        }))
    );

    assert_eq!(
        res_send_funds_success.messages[1],
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: String::from(new_user.sender),
            amount: vec![coin(10, "uluna")],
        }))
    );
}
