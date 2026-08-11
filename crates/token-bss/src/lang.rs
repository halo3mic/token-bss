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

    /// Storage location for nested mappings like ERC20 `allowances[owner][spender]`.
    pub fn nested_mapping_loc(&self, slot: B256, owner: Address, spender: Address) -> B256 {
        let inner = self.mapping_loc(slot, owner);
        self.mapping_loc(inner, spender)
    }

    pub fn solidity_mapping_loc(storage_index: &FixedBytes<32>, key: &FixedBytes<32>) -> B256 {
        Self::mapping_loc_from_tokens(key, storage_index)
    }
    
    pub fn vyper_mapping_loc(storage_index: &FixedBytes<32>, key: &FixedBytes<32>) -> B256 {
        Self::mapping_loc_from_tokens(storage_index, key)
    }

    fn mapping_loc_from_tokens(token_0: &FixedBytes<32>, token_1: &FixedBytes<32>) -> B256 {
        let hashable = [token_0.0.to_vec(), token_1.0.to_vec()].concat();
        alloy_utils::keccak256(hashable)
    }

}

impl std::fmt::Display for EvmLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvmLanguage::Solidity => write!(f, "solidity"),
            EvmLanguage::Vyper => write!(f, "vyper"),
        }
    }
}

impl std::str::FromStr for EvmLanguage {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "solidity" => Ok(EvmLanguage::Solidity),
            "vyper" => Ok(EvmLanguage::Vyper),
            _ => Err(eyre::eyre!("Invalid language")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_mapping_loc_solidity_matches_double_hash() {
        let slot = B256::from(U256::from(10));
        let owner: Address = "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5".parse().unwrap();
        let spender: Address = "0x1111111111111111111111111111111111111111".parse().unwrap();

        let lang = EvmLanguage::Solidity;
        let expected = lang.mapping_loc(lang.mapping_loc(slot, owner), spender);
        assert_eq!(lang.nested_mapping_loc(slot, owner, spender), expected);
    }

    #[test]
    fn nested_mapping_loc_vyper_matches_double_hash() {
        let slot = B256::from(U256::from(1));
        let owner: Address = Address::from_word(B256::from(U256::from(1)));
        let spender: Address = Address::from_word(B256::from(U256::from(2)));

        let lang = EvmLanguage::Vyper;
        let expected = lang.mapping_loc(lang.mapping_loc(slot, owner), spender);
        assert_eq!(lang.nested_mapping_loc(slot, owner, spender), expected);
    }
}