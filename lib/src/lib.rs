pub mod crypto;
pub mod error;
pub mod network;
pub mod sha256;
pub mod types;
pub mod util;

use serde::{Deserialize, Serialize};
use uint::construct_uint;

construct_uint! {
    // construct an unsigned 256-bit integer
    // 4 x 64bit
    #[derive(Serialize, Deserialize)]
    pub struct U256(4);
}

/// initial reward in bitcoin - multiply by 10^8 to get satoshis
pub const INITIAL_REWARD: u64 = 50;
/// Halving interval in blocks
pub const HALVING_INTERVAL: u64 = 210;
/// Ideal block time in seconds
pub const IDEAL_BLOCK_TIME: u64 = 10;
pub const DIFFICULTY_UPDATE_INTERVAL: u64 = 50;
/// maximum mempool transaction age in seconds
pub const MAX_MEMPOOL_TRANSACTION_AGE: u64 = 600;
pub const MIN_TARGET: U256 = U256([
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_00FF_FFFF_FFFF,
]);
