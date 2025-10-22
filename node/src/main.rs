use anyhow::Result;
use argh::*;
use btclib::types::Blockchain;
use dashmap::DashMap;
use static_init::dynamic;
use std::path::Path;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

mod handler;
mod util;

#[dynamic]
pub static BLOCKCHAIN: RwLock<Blockchain> = RwLock::new(Blockchain::new());

#[dynamic]
/// Node pool
pub static NODES: DashMap<String, TcpStream> = DashMap::new();

#[derive(FromArgs, Debug)]
/// A toy blockchain node :D
struct Args {
    #[argh(option, default = "9000")]
    /// port number
    port: u16,
    #[argh(option, default = "String::from(\"./blockchain.cbor\")")]
    /// blockchain file location
    blockchain_file: String,
    #[argh(positional)]
    /// addresses of inital nodes
    nodes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();
    let port = args.port;
    let blockchain_path = args.blockchain_file;
    let nodes = args.nodes;

    if Path::new(&blockchain_path).exists() {
        util::load_blockchain(&blockchain_path).await?;
    } else {
        util::populate_connections(&nodes).await?;
        println!("total amount of known nodes: {}", NODES.len());

        if nodes.is_empty() {
            println!("no initial nodes provided, starting as a seed node");
        } else {
            let (longest_name, longest_count) = util::find_longest_chain_node().await?;
            // download blockchain from the node with the longest blockchain
            util::download_blockchain(&longest_name, longest_count).await?;
            println!("blockchain downloaded from: {longest_name}");
            //recalculate utxos
            let mut blockchain = BLOCKCHAIN.write().await;
            blockchain.rebuild_utxos();
            drop(blockchain);
            let mut blockchain = BLOCKCHAIN.write().await;
            blockchain.try_adjust_target();
            drop(blockchain);
        }
    }

    // tasks
    tokio::spawn(util::cleanup());
    tokio::spawn(util::save(blockchain_path.clone()));

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {addr}");
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(handler::handle_connection(socket));
    }
}
