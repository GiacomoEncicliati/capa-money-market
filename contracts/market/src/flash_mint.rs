use cosmwasm_bignumber::math::Uint256;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Binary, Response, CosmosMsg, WasmMsg, attr, to_binary};
use cw20::Cw20ExecuteMsg;
use moneymarket::market::ExecuteMsg;

use crate::{error::ContractError, state::read_config};

pub fn flash_mint(    
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg_callback:Binary,
    amount:Uint256,
) -> Result<Response, ContractError> {

    // load config
    let config = read_config(deps.storage)?;

    // compute fee amount
    let fee_amount = config.flash_mint_fee * amount;

    let mut messages:Vec<CosmosMsg> = vec![];

    // insert mint msg
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.stable_contract.to_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            recipient: info.sender.to_string(),
            amount: amount.into(),
        })?,
    }));

    // insert callback msg
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: info.sender.to_string(),
        funds: vec![],
        msg: msg_callback,
    }));

    // insert private flahs end msg
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::PrivateFlashEnd {
            flash_minter: info.sender.to_string(),
            burn_amount: amount,
            fee_amount: fee_amount,
         })?,
    }));

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "flash_mint"),
        attr("flash_minter", info.sender),
        attr("amount", amount),
        attr("fee_amount", fee_amount)
        ]
    ))

}

pub fn private_flash_end(    
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    flash_minter: String,
    burn_amount: Uint256,
    fee_amount: Uint256,
) -> Result<Response, ContractError> {

        // the sender must be the contract itself
        if info.sender != env.contract.address {
            return Err(ContractError::Unauthorized {});
        }

        let config = read_config(deps.storage)?;

        let mut messages:Vec<CosmosMsg> = vec![];

        // insert msg burn
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.stable_contract.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
                owner: flash_minter.to_string(),
                amount: burn_amount.into()
            })?,
        }));

        // insert msg fee transfer to collector only if fee_amount > 0
        if fee_amount > Uint256::zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.stable_contract.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: flash_minter,
                    recipient: config.collector_contract.to_string(),
                    amount: fee_amount.into()
                })?,
            }));
        }

        Ok(Response::new().add_messages(messages).add_attribute("action", "private_flash_end"))

}