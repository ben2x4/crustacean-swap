use cosmwasm_std::{
    entry_point, to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Coin, Uint128, Addr, WasmMsg, CosmosMsg
};
use cw20_base::state::{TOKEN_INFO, BALANCES};
use cw20_base::contract::{instantiate as cw20_instantiate, execute_mint,query_balance};
use cw20_base;
use cw20::{Cw20ExecuteMsg, MinterResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, CountResponse};
use crate::state::{State, STATE};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        nativeSupply: Coin{denom:msg.nativeDenom, amount: Uint128(0)},
        tokenAddress: msg.tokenAddress,
        tokenSupply: Uint128(0),
    };
    STATE.save(deps.storage, &state)?;

    cw20_instantiate(deps,_env.clone(),info,cw20_base::msg::InstantiateMsg{name:"liquidity".into(),symbol:"AAAA".into(),decimals:0,initial_balances:vec![],mint:Some(MinterResponse{minter:_env.contract.address.clone().into(), cap: None})})?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddLiquidity {tokenAmount} => try_add_liquidity(deps, info, _env, tokenAmount),
    }
}

pub fn try_add_liquidity(deps: DepsMut, info: MessageInfo, _env: Env, token_amount: Uint128) -> Result<Response, ContractError> {

    let state = STATE.load(deps.storage).unwrap();

     // create transfer cw20 msg
     let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.clone().into(),
        recipient: _env.contract.address.clone().into(),
        amount: token_amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: state.tokenAddress.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        send: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.tokenSupply += token_amount;
        state.nativeSupply.amount += info.funds[0].amount.clone();
        Ok(state)
    })?;

    let token = TOKEN_INFO.load(deps.storage)?;


    if token.total_supply == Uint128(0) {
        let mint_amount = info.funds[0].clone().amount;
        let sub_info = MessageInfo {
            sender: _env.contract.address.clone(),
            funds: vec![],
        };
        execute_mint(deps, _env, sub_info, info.sender.clone().into(), mint_amount)?;
    }

    Ok(Response {
        messages: vec![cw20_transfer_cosmos_msg],
        submessages: vec![],
        attributes: vec![],
        data: None,
    })
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
        QueryMsg::Balance {address} => to_binary(&query_balance(deps, address)?)
    }
}

fn query_count(deps: Deps) -> StdResult<CountResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(CountResponse { count: 1})
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { nativeDenom: "test".to_string(), tokenAddress: Addr::unchecked("asdf")};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn add_liqudity() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = InstantiateMsg { nativeDenom: "test".to_string(), tokenAddress: Addr::unchecked("asdf")};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::AddLiquidity {tokenAmount: Uint128(1) };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
