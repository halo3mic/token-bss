use ethers::abi::{Tokenizable, Token};
use ethers::prelude::*;
use eyre::Result;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmLanguage {
    Solidity,
    Vyper,
}

impl EvmLanguage {

    pub fn mapping_loc(&self, slot: H256, holder: H160) -> H256 {
         match &self {
            EvmLanguage::Solidity => Self::solidity_mapping_loc(&slot, &H256::from(holder)),
            EvmLanguage::Vyper => Self::vyper_mapping_loc(&slot, &H256::from(holder)),
        }
    }

   pub fn solidity_mapping_loc(storage_index: &H256, key: &H256) -> H256 {
        Self::mapping_loc_from_tokens(key.into_token(), storage_index.into_token())
    }
    
    pub fn vyper_mapping_loc(storage_index: &H256, key: &H256) -> H256 {
        Self::mapping_loc_from_tokens(storage_index.into_token(), key.into_token())
    }

    fn mapping_loc_from_tokens(token_0: Token, token_1: Token) -> H256 {
        let hash_input = ethers::abi::encode(&[ token_0, token_1 ]);
        ethers::utils::keccak256(hash_input).into()
    }

    pub fn to_string(&self) -> String {
        match &self {
            EvmLanguage::Solidity => String::from("solidity"),
            EvmLanguage::Vyper => String::from("vyper"),
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "solidity" => Ok(EvmLanguage::Solidity),
            "vyper" => Ok(EvmLanguage::Vyper),
            _ => Err(eyre::eyre!("Invalid language")),
        }
    }

}