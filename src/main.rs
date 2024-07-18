use std::sync::Arc;

use alloy::{ providers::{Provider, ProviderBuilder, WsConnect}};
use continue_fuzz::helper::call_tracer;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
 
    // Create a provider.
    let http_rpc = "http://64.71.166.16/eth-chain";
    let wss_rpc = "wss://64.71.166.16/eth-chain";
    let wss_rpc = "wss://eth.merkle.io";

    let ws = WsConnect::new(wss_rpc);
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let http_rpc = http_rpc.parse()?;
    let http_provider = ProviderBuilder::new().on_http(http_rpc);


    // // Set up the IPC transport which is consumed by the RPC client.
    // let ipc_path = "/tmp/reth.ipc";

    // // Create the provider.
    // let ipc = IpcConnect::new(ipc_path.to_string());
    // let provider = ProviderBuilder::new().on_ipc(ipc).await?;

    // Subscribe to blocks.
    let mut subscription = provider.subscribe_blocks().await?;
    
    // let mut stream = subscription.into_stream().take(2);

    // set tx_hash_queue
    let http_provider = Arc::new(http_provider);

    loop {
        match subscription.recv().await {
            Ok(block) => {
                let block_hash = block.header.hash.unwrap();
                println!("block hash is {:?}",block_hash );

                let block = provider.get_block_by_hash(block_hash, alloy_rpc_types::BlockTransactionsKind::Full).await?.unwrap();
                let block_tx_list = block.transactions.as_transactions().unwrap();
                let mut tx_queue = vec![];
                for tx in block_tx_list {
                    if tx.to.is_some() {
                        tx_queue.push(tx);
                    }
                }
                for tx in tx_queue {
                    let tx_hash = tx.hash;
                    let result = call_tracer(http_provider.clone(), tx_hash).await.unwrap();
                    if !result.is_empty() {
                        // println!("{:?}", result);
                    }
                }

            },
            Err(e) => {
                eprintln!("Error receiving block: {:?}", e);
                break; 
            }
        }
    }

    Ok(())
}
