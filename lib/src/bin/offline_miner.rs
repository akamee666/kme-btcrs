use btclib::types::Block;
use btclib::util::Saveable;
use std::{env, process::exit};

fn main() {
    // parse block path and setps counts from the first and second argument respectively
    let (path, steps) = if let (Some(arg), Some(arg2)) = (env::args().nth(1), env::args().nth(2)) {
        (arg, arg2)
    } else {
        eprintln!("Usage: miner <block_file> <steps>");
        exit(1);
    };

    // parse step counts
    let steps: usize = if let Ok(s @ 1..=usize::MAX) = steps.parse() {
        s
    } else {
        eprintln!("<steps> should be a positive integer");
        exit(1);
    };

    let og_block = Block::load_from_file(path).expect("Failed to load block file");
    let mut block = og_block.clone();

    let mut counter = 1;
    while !block.header.mine(steps) {
        println!("mining....counter: {counter}");
        counter += 1;
    }
    let reward = &block.transactions[0].outputs[0].value / 100_000_000;

    println!("Block mined! number of attempts: {}", block.header.nonce);
    println!("Rewarded: {reward} BTC");
    println!("hash was: {}", og_block.header.hash());
    println!("mined hash: {}", block.header.hash());
    // println!("original block: {:#?}", og_block);
    // Header = version || prev_hash || merkle_root || timestamp || target || nonce
    // Hash   = SHA256(SHA256(Header))
    // After mining
    // println!("mined block: {:#?}", block);
}
