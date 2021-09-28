use crate::contract::{execute, instantiate, query};
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{GenericBalance, State, Wager};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    coin, coins, from_binary, to_binary, Addr, BankMsg, CosmosMsg, SubMsg, Uint128, WasmMsg,
};
use cw20::{Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

#[test]
fn test_initialization() {
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    //check if the initialization works by unwrapping
    let initialization_check = instantiate(deps.as_mut(), mock_env(), creator, inst_msg).unwrap();
    assert_eq!(0, initialization_check.messages.len());

    //check if state matches sender
    let res_query_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: State = from_binary(&res_query_config).unwrap();

    assert_eq!("creator", config.creator.as_str());
}

#[test]
fn test_execute_create_wager_native() {
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    let _initialization_check = instantiate(deps.as_mut(), mock_env(), creator, inst_msg).unwrap();

    let wager_id = String::from("test_id");
    let new_user = mock_info("new_user", &coins(10, "uluna"));

    let _res_create_wager = execute(
        deps.as_mut(),
        mock_env(),
        new_user,
        ExecuteMsg::CreateWagerNative {
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
fn test_execute_create_wager_cw20() {
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check = instantiate(deps.as_mut(), mock_env(), creator, inst_msg).unwrap();

    let token_contract = mock_info("cw20-token", &[]);

    let wager_id = String::from("test_id");

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "new_user".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::CreateWager {
            wager_id: wager_id.clone(),
        })
        .unwrap(),
    });

    let _res = execute(deps.as_mut(), mock_env(), token_contract, msg).unwrap();

    let res_query_wager =
        query(deps.as_ref(), mock_env(), QueryMsg::Wager { id: wager_id }).unwrap();

    let wager: Wager = from_binary(&res_query_wager).unwrap();

    assert_eq!("creator", wager.arbiter);
    assert_eq!("new_user", wager.user1);
    assert_eq!("empty", wager.user2);

    let test_user1_balance = GenericBalance {
        native: vec![],
        cw20: vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20-token"),
            amount: Uint128::new(100),
        }],
    };

    assert_eq!(test_user1_balance, wager.user1_balance);
    assert_eq!(GenericBalance::new(), wager.user2_balance);
}

#[test]
fn test_execute_add_funds_cw20() {
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check = instantiate(deps.as_mut(), mock_env(), creator, inst_msg).unwrap();

    let token_contract = mock_info("cw20-token", &[]);

    let wager_id = String::from("test_id");

    let create_wager_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "new_user".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::CreateWager {
            wager_id: wager_id.clone(),
        })
        .unwrap(),
    });

    let _res = execute(
        deps.as_mut(),
        mock_env(),
        token_contract.clone(),
        create_wager_msg,
    )
    .unwrap();

    let add_funds_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "new_user2".to_string(),
        amount: Uint128::from(99u128),
        msg: to_binary(&Cw20HookMsg::AddFunds {
            wager_id: wager_id.clone(),
        })
        .unwrap(),
    });

    let res_add_funds_unsuccessfully = execute(
        deps.as_mut(),
        mock_env(),
        token_contract.clone(),
        add_funds_msg,
    );
    assert!(res_add_funds_unsuccessfully.is_err());

    let test_user1_balance = GenericBalance {
        native: vec![],
        cw20: vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20-token"),
            amount: Uint128::new(100),
        }],
    };

    let add_funds_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "new_user2".to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::AddFunds {
            wager_id: wager_id.clone(),
        })
        .unwrap(),
    });
    let _res_add_funds_successfully =
        execute(deps.as_mut(), mock_env(), token_contract, add_funds_msg).unwrap();

    let res_query_wager =
        query(deps.as_ref(), mock_env(), QueryMsg::Wager { id: wager_id }).unwrap();

    let wager: Wager = from_binary(&res_query_wager).unwrap();

    let test_user2_balance = GenericBalance {
        native: vec![],
        cw20: vec![Cw20CoinVerified {
            address: Addr::unchecked("cw20-token"),
            amount: Uint128::new(100),
        }],
    };
    assert_eq!("creator", wager.arbiter);
    assert_eq!("new_user", wager.user1);
    assert_eq!("new_user2", wager.user2);
    assert_eq!(test_user1_balance, wager.user1_balance);
    assert_eq!(test_user2_balance, wager.user2_balance);
    assert_eq!(wager.user1_balance, wager.user2_balance);
}
#[test]
fn test_execute_cancel_wager() {
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check = instantiate(deps.as_mut(), mock_env(), creator, inst_msg).unwrap();

    let new_user = mock_info("new_user", &coins(10, "uluna"));

    let wager_id = String::from("test_id");

    let _res_create_wager = execute(
        deps.as_mut(),
        mock_env(),
        new_user.clone(),
        ExecuteMsg::CreateWagerNative {
            wager_id: wager_id.clone(),
        },
    )
    .unwrap();

    let sneaky_user = mock_info("sneaky_user", &[]);

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
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check =
        instantiate(deps.as_mut(), mock_env(), creator.clone(), inst_msg).unwrap();

    let new_user = mock_info("new_user", &coins(10, "uluna"));

    let wager_id = String::from("test_id");

    let _res_create_wager = execute(
        deps.as_mut(),
        mock_env(),
        new_user.clone(),
        ExecuteMsg::CreateWagerNative {
            wager_id: wager_id.clone(),
        },
    )
    .unwrap();

    let new_user2 = mock_info("new_user2", &coins(10, "uluna"));

    let _res_add_funds = execute(
        deps.as_mut(),
        mock_env(),
        new_user2,
        ExecuteMsg::AddFundsNative {
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
        creator,
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

#[test]
fn test_execute_send_funds_cw20() {
    let creator = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: creator.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check =
        instantiate(deps.as_mut(), mock_env(), creator.clone(), inst_msg).unwrap();

    let token_contract = mock_info("cw20-token", &[]);

    let wager_id = String::from("test_id");

    let new_user = mock_info("new_user", &[]);
    let new_user2 = mock_info("new_user2", &[]);

    let create_wager_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: new_user.sender.to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::CreateWager {
            wager_id: wager_id.clone(),
        })
        .unwrap(),
    });

    let _res = execute(
        deps.as_mut(),
        mock_env(),
        token_contract.clone(),
        create_wager_msg,
    )
    .unwrap();

    let add_funds_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: new_user2.sender.to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::AddFunds {
            wager_id: wager_id.clone(),
        })
        .unwrap(),
    });

    let _res_add_funds_successfully =
        execute(deps.as_mut(), mock_env(), token_contract, add_funds_msg).unwrap();

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
        creator,
        ExecuteMsg::SendFunds {
            wager_id: wager_id.clone(),
            winner_address: new_user.sender.clone(),
        },
    )
    .unwrap();

    // Wager was cancelled.
    assert!(query(deps.as_ref(), mock_env(), QueryMsg::Wager { id: wager_id }).is_err());

    let send_msg = Cw20ExecuteMsg::Transfer {
        recipient: String::from(new_user.sender),
        amount: Uint128::new(100),
    };

    let expected_msg = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: String::from("cw20-token"),
        msg: to_binary(&send_msg).unwrap(),
        funds: vec![],
    }));

    assert_eq!(res_send_funds_success.messages[0], expected_msg);
    assert_eq!(res_send_funds_success.messages[1], expected_msg);
}
