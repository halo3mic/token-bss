use alloy::rpc::types::trace::geth::StructLog;
use std::collections::{HashMap, HashSet};
use super::lang::EvmLanguage;
use crate::common::*;


#[derive(Default)]
pub struct TraceParser {
    depth_to_address: HashMap<usize, Address>,
    hashed_vals: HashMap<B256, (B256, B256)>,
    results: HashSet<(Address, B256, EvmLanguage)>,
    holder: Address,
    spender: Option<Address>,
}

impl TraceParser {

    pub fn parse(struct_logs: Vec<StructLog>, token: Address, holder: Address) -> Result<Vec<(Address, B256, EvmLanguage)>> {
        let mut parser = TraceParser::default();
        parser.set_holder(holder);
        parser.depth_to_address.insert(1, token);
        parser.parse_logs(struct_logs)?;
        Ok(parser.results.into_iter().collect())
    }

    /// Parse a trace of `allowance(owner, spender)` for the base storage slot of the nested mapping.
    pub fn parse_allowance(
        struct_logs: Vec<StructLog>,
        token: Address,
        owner: Address,
        spender: Address,
    ) -> Result<Vec<(Address, B256, EvmLanguage)>> {
        let mut parser = TraceParser::default();
        parser.set_holder(owner);
        parser.spender = Some(spender);
        parser.depth_to_address.insert(1, token);
        parser.parse_logs(struct_logs)?;
        Ok(parser.results.into_iter().collect())
    }

    fn parse_logs(&mut self, struct_logs: Vec<StructLog>) -> Result<()>   {
        for log in struct_logs {
            self.parse_log(log)?;
        }
        Ok(())
    }

    fn parse_log(&mut self, log: StructLog) -> Result<()> {
        let depth = log.depth as usize;
        match log.op.as_str() {
            "SLOAD" => self.parse_sload(log, depth)?,
            "KECCAK256" => self.parse_sha3(log)?,
            "STATICCALL" | "CALL" => self.parse_call(log, depth)?,
            "DELEGATECALL" => self.parse_delegatecall(depth)?,
            _ => (),
        }   
        Ok(())
    }

    fn parse_sload(&mut self, log: StructLog, depth: usize) -> Result<()> {
        if log.memory.as_ref().map(|m| m.len() < 2).unwrap_or(true) || log.stack.as_ref().is_none() {
            return Ok(());
        }
        let stack = log.stack.as_ref().unwrap();
        let slot_idx: B256 = stack[stack.len()-1].into();
        let Some((hashed_val_0, hashed_val_1)) = self.hashed_vals.get(&slot_idx).copied() else {
            return Ok(());
        };

        if let Some(spender) = self.spender {
            self.match_allowance_sload(depth, hashed_val_0, hashed_val_1, spender);
        } else {
            self.match_balance_sload(depth, hashed_val_0, hashed_val_1);
        }

        Ok(())
    }

    fn match_balance_sload(&mut self, depth: usize, hashed_val_0: B256, hashed_val_1: B256) {
        let (slot, lang) = match &self.holder.into_word() {
            v if *v == hashed_val_0 => (hashed_val_1, EvmLanguage::Solidity),
            v if *v == hashed_val_1 => (hashed_val_0, EvmLanguage::Vyper),
            _ => return,
        };
        let contract = self.depth_to_address.get(&depth).unwrap();
        self.results.insert((*contract, slot, lang));
    }

    /// Nested mapping: allowances[owner][spender]
    /// Solidity: keccak256(spender || keccak256(owner || slot))
    /// Vyper:    keccak256(keccak256(slot || owner) || spender)
    fn match_allowance_sload(
        &mut self,
        depth: usize,
        hashed_val_0: B256,
        hashed_val_1: B256,
        spender: Address,
    ) {
        let spender_word = spender.into_word();
        let owner_word = self.holder.into_word();

        // Solidity: outer = keccak(spender, inner)
        if hashed_val_0 == spender_word {
            if let Some((inner_0, inner_1)) = self.hashed_vals.get(&hashed_val_1).copied() {
                if inner_0 == owner_word {
                    let contract = self.depth_to_address.get(&depth).unwrap();
                    self.results.insert((*contract, inner_1, EvmLanguage::Solidity));
                }
            }
        }

        // Vyper: outer = keccak(inner, spender)
        if hashed_val_1 == spender_word {
            if let Some((inner_0, inner_1)) = self.hashed_vals.get(&hashed_val_0).copied() {
                if inner_1 == owner_word {
                    let contract = self.depth_to_address.get(&depth).unwrap();
                    self.results.insert((*contract, inner_0, EvmLanguage::Vyper));
                }
            }
        }
    }

    fn parse_sha3(&mut self, log: StructLog) -> Result<()> {
        // Anvil/geth may return memory words with or without a `0x` prefix.
        let memory_hex: String = log.memory.as_ref()
            .expect("SHA3 op should have memory content")
            .iter()
            .map(|word| word.strip_prefix("0x").unwrap_or(word.as_str()))
            .collect();
        let memory = hex::decode(memory_hex)?;
        let stack = log.stack.as_ref()
            .expect("SHA3 op should have stack content");
        let mem_offset = stack[stack.len()-1].to::<usize>();
        let mem_length = stack[stack.len()-2].to::<usize>();
        if mem_length == 64 { // Only concerned about storage mappings
            let hashed_val = memory[mem_offset..mem_offset+mem_length].to_vec();
            let hash = alloy_utils::keccak256(&hashed_val);
            let hashed_val_0 = B256::from_slice(&hashed_val[0..32]);
            let hashed_val_1 = B256::from_slice(&hashed_val[32..64]);
            self.hashed_vals.insert(hash, (hashed_val_0, hashed_val_1));
        }
        Ok(())
    }

    fn parse_call(&mut self, log: StructLog, depth: usize) -> Result<()> {
        let stack = log.stack.expect("CALL op should have stack content");
        let address = stack[stack.len()-2].to::<U160>().into();
        self.depth_to_address.insert(depth + 1, address);
        Ok(())
    }

    fn parse_delegatecall(&mut self, depth: usize) -> Result<()> {
        let prev_address = *self.depth_to_address.get(&depth).unwrap();
        self.depth_to_address.insert(depth + 1, prev_address);
        Ok(())
    }

    fn set_holder(&mut self, holder: Address) {
        self.holder = holder;
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sha3_accepts_0x_prefixed_memory_words() {
        let mut keccak = StructLog::default();
        keccak.op = "KECCAK256".into();
        keccak.depth = 1;
        keccak.memory = Some(vec![
            "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
            "0x0000000000000000000000000000000000000000000000000000000000000002".into(),
        ]);
        // stack top = offset, next = length
        keccak.stack = Some(vec![U256::from(64), U256::from(0)]);

        TraceParser::parse(vec![keccak], Address::ZERO, Address::ZERO).unwrap();
    }
}
