use ethers::{
    types::{H160, H256, U256, Bytes},
};
use ethers::abi::{
    ethereum_types::BigEndianHash,
    Tokenizable
};

pub fn bytes_to_h256(val: Bytes) -> H256 {
    let bytes = val.to_vec();
    if bytes.len() == 0 {
        H256::zero()
    } else {
        H256::from_slice(&bytes)
    }
}

pub fn u256_to_h256(val: U256) -> H256 {
    H256::from_uint(&val)
}

pub fn h256_to_u256(val: H256) -> U256 {
    H256::into_uint(&val)
}