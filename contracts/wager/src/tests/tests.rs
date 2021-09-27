use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{GenericBalance, State, Wager};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, from_binary};
use cw20::Balance;

#[test]
fn test_initialization() {
    let info = mock_info("creator", &[]);
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: info.clone().sender,
    };

    //check if the initialization works by unwrapping
    let _initialization_check = instantiate(deps.as_mut(), mock_env(), info, inst_msg).unwrap();
    assert_eq!(0, _initialization_check.messages.len());

    //check if state matches sender
    let res_query_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: State = from_binary(&res_query_config).unwrap();

    assert_eq!("creator", config.creator.as_str());
    assert_eq!("creator", config.owner.as_str());
}

#[test]
fn test_execute_create_wager_native() {
    let info = mock_info("creator", &coins(0, "luna"));
    let mut deps = mock_dependencies(&[]);

    let inst_msg = InstantiateMsg {
        sender: info.clone().sender,
    };

    //check if the initialization works by unwrapping
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

    let test_user2_balance = GenericBalance::new();

    assert_eq!(test_user1_balance, wager.user1_balance);
    assert_eq!(test_user2_balance, wager.user2_balance);
}
