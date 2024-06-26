use crate::common::*;


const DEFAULT_PRECISION_MUL: u128 = 10_000;

pub fn ratio_f64(val1: U256, val2: U256, precision_mul: Option<u128>) -> f64 {
    if val2 == U256::ZERO {
        return f64::INFINITY;
    }
    let p_mul = precision_mul.unwrap_or(DEFAULT_PRECISION_MUL);
    let ur_bn = U512::from(val1) * U512::from(p_mul) / U512::from(val2);
    if ur_bn <= U512::from(U128::MAX) {
        ur_bn.to::<u128>() as f64 / p_mul as f64
    } else {
        f64::INFINITY
    }
}

pub fn bytes_to_u256(val: Bytes) -> U256 {
    let bytes = val.to_vec();
    if bytes.is_empty() {
        U256::ZERO
    } else {
        B256::from_slice(&bytes[..32]).into()
    }
}
