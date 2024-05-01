use crate::common::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmLanguage {
    Solidity,
    Vyper,
}

impl EvmLanguage {

    pub fn mapping_loc(&self, slot: B256, holder: Address) -> B256 {
        let holder: B256 = holder.into_word(); 
         match &self {
            EvmLanguage::Solidity => Self::solidity_mapping_loc(&slot, &holder),
            EvmLanguage::Vyper => Self::vyper_mapping_loc(&slot, &holder),
        }
    }

    pub fn solidity_mapping_loc(storage_index: &FixedBytes<32>, key: &FixedBytes<32>) -> B256 {
        Self::mapping_loc_from_tokens(key, storage_index)
    }
    
    pub fn vyper_mapping_loc(storage_index: &FixedBytes<32>, key: &FixedBytes<32>) -> B256 {
        Self::mapping_loc_from_tokens(storage_index, key)
    }

    fn mapping_loc_from_tokens(token_0: &FixedBytes<32>, token_1: &FixedBytes<32>) -> B256 {
        let hashable = vec![token_0.0.to_vec(), token_1.0.to_vec()].concat();
        alloy_utils::keccak256(&hashable).into()
    }

    pub fn to_string(&self) -> String {
        match &self {
            EvmLanguage::Solidity => String::from("solidity"),
            EvmLanguage::Vyper => String::from("vyper"),
        }
    }

    // pub fn from_str(s: &str) -> Result<Self> {
    //     match s.to_lowercase().as_str() {
    //         "solidity" => Ok(EvmLanguage::Solidity),
    //         "vyper" => Ok(EvmLanguage::Vyper),
    //         _ => Err(eyre::eyre!("Invalid language")),
    //     }
    // }

}