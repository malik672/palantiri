use  palantiri::{rpc::RpcClient, transport::http::TransportBuilder};

#[tokio::main]
async fn main() {
    let rpc = RpcClient::new(
        TransportBuilder::new(
            "https://mainnet.infura.io/v3/1f2bd7408b1542e89bd4274b688aa6a4".to_string(),
        )
        .build_http(),
    );

    let no = 
    22525109;

    let s = rpc.get_block_by_number(no, true).await;

    println!("RPC Client: {:?}", s);
}
