use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_std::coins;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

#[test]
fn ooga() {
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
    assert_eq!("creator", res_query_config.creator.as_str());
    assert_eq!("creator", res_query_config.owner.as_str());
}
