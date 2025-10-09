pub mod crypto;
pub mod sha256;
pub mod types;
pub mod util;

use uint::construct_uint;

construct_uint! {
    // construct an unsigned 256-bit integer
    // 4 x 64bit
    pub struct U256(4);
}
