use std::{collections::{HashMap, HashSet}, sync::Arc};

use alloy::{hex::decode, primitives::{Address, Bytes, FixedBytes}, providers::{ext::DebugApi, Provider, RootProvider}, sol_types::SolValue, transports::http::{Client, Http}};
use alloy_rpc_types::trace::geth::{CallFrame, GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions, GethDefaultTracingOptions, GethTrace};
use evmole::function_arguments;
use eyre::Result;
use alloy_dyn_abi::{DynSolType, DynSolValue};
use foundry_common;

fn dfs_collect_calls(frame: &CallFrame, result: &mut Vec<(Address, Bytes)>) {
    if let Some(to_addr) = &frame.to {
        result.push((to_addr.clone(), frame.input.clone()));
    }
    for call in &frame.calls {
        dfs_collect_calls(call, result);
    }
}

pub async fn call_tracer(http_provider: Arc<RootProvider<Http<Client>>>, tx_hash: FixedBytes<32>) -> Result<Vec<(Address, Bytes)>> {
       // call_tracer 
       let call_options = GethDebugTracingOptions {
        config: GethDefaultTracingOptions {
            disable_storage: Some(true),
            enable_memory: Some(false),
            ..Default::default()
        },
        tracer: Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer)),
        ..Default::default()
    };
    let call_result = http_provider.debug_trace_transaction(tx_hash, call_options).await?;
    
    let call_result = if let GethTrace::CallTracer(ref call_frame) = call_result {
        Some(call_frame)
    } else {
        None
    }.unwrap();

    let mut calls = Vec::new();
    dfs_collect_calls(call_result, &mut calls);
    let mut ret_calls = Vec::new();
    for sub_call in calls {
        if sub_call.1.len() > 4 {
            ret_calls.push(sub_call);
        }
    }
   
    Ok(ret_calls)  

}

pub async fn get_tx_constant(http_provider: Arc<RootProvider<Http<Client>>>, addr_calldata: Vec<(Address, Bytes)>)->Result<()> {
    // code cache 
    let mut code_cache: HashMap<Address, Vec<u8>> = HashMap::new();

    // constant set
    let mut constant_set: HashSet<String> = HashSet::new();

    for (address, ref calldata) in addr_calldata {
        let code = if code_cache.contains_key(&address) {
            code_cache.get_mut(&address).unwrap().to_owned()
        } else {
            let code = http_provider.get_code_at(address).await?.to_vec();
            code_cache.insert(address, code.clone());
            code
        };
       
        let selector: &[u8; 4] = &calldata.to_vec().get(0..4).unwrap().try_into().unwrap();
        let arguments: String = function_arguments(&code, selector, 0);
        if !arguments.is_empty() {
            println!("arguments is {}", arguments);
            let abi: DynSolType = arguments.as_str().parse().unwrap();
            println!("abi is {:?}", abi);
            let data = calldata.get(4..).unwrap();
            let decoded = abi.abi_decode_sequence(data).unwrap();
            println!("{:?}", decoded);
            
        }
 
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::providers::{ProviderBuilder};
    use foundry_common::{
        abi::{encode_function_args}
    };
    use super::*;

    #[tokio::test]
    async fn test_call_tracer() -> Result<()> {
        // Create a provider.
        let http_rpc = "http://64.71.166.16/eth-chain";
        let euler_hash = "0xc310a0affe2169d1f6feec1c63dbc7f7c62a887fa48795d327d4d2da2d6b111d";

        let http_rpc = http_rpc.parse()?;
        let http_provider = ProviderBuilder::new().on_http(http_rpc);
        let tx_hash = FixedBytes::from_str(euler_hash).unwrap();

        let result = call_tracer(Arc::new(http_provider), tx_hash).await.unwrap();
        println!("ret_calls: {:?}", result);
        Ok(())
       
    }


    #[tokio::test]
    async fn test_get_tx_constant() -> Result<()> {
        // Create a provider.
        let http_rpc = "http://64.71.166.16/eth-chain";
        let euler_hash = "0xc310a0affe2169d1f6feec1c63dbc7f7c62a887fa48795d327d4d2da2d6b111d";

        let http_rpc = http_rpc.parse()?;
        let http_provider = Arc::new(ProviderBuilder::new().on_http(http_rpc));
        let tx_hash = FixedBytes::from_str(euler_hash).unwrap();

        let result = call_tracer(http_provider.clone(), tx_hash).await.unwrap();
        get_tx_constant(http_provider, result).await?;
        foundry_common::abi::abi_decode_calldata();
        Ok(())
    }
}