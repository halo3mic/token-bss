use std::collections::{ HashMap, HashSet };
use ethers::types::{StructLog, H256, H160};
use eyre::Result;
use super::lang::EvmLanguage;
use crate::conversion as c;


#[derive(Default)]
pub struct TraceParser {
    depth_to_address: HashMap<usize, H160>,
    hashed_vals: HashMap<H256, (H256, H256)>,
    results: HashSet<(H160, H256, EvmLanguage)>,
    holder: H160,
}

impl TraceParser {

    pub fn parse(struct_logs: Vec<StructLog>, token: H160, holder: H160) -> Result<Vec<(H160, H256, EvmLanguage)>> {
        let mut parser = TraceParser::default();
        parser.holder = holder;
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
            "SHA3" => self.parse_sha3(log)?,
            "STATICCALL" | "CALL" => self.parse_call(log, depth)?,
            "DELEGATECALL" => self.parse_delegatecall(depth)?,
            _ => (),
        }   
        Ok(())
    }

    fn parse_sload(&mut self, log: StructLog, depth: usize) -> Result<()> {
        if log.memory.as_ref().map(|m| m.len() < 2).unwrap_or(true) || 
            log.stack.as_ref().is_none() || 
            log.storage.as_ref().is_none() 
        {
            return Ok(());
        }
        for slot_idx in log.storage.as_ref().unwrap().keys() {
            if let Some((hashed_val_0, hashed_val_1)) = self.hashed_vals.get(slot_idx) {
                let (slot, lang) = match &H256::from(self.holder) {
                    v if *v == *hashed_val_0 => (*hashed_val_1, EvmLanguage::Solidity),
                    v if *v == *hashed_val_1 => (*hashed_val_0, EvmLanguage::Vyper),
                    _ => return Ok(()),
                };
                let contract = self.depth_to_address.get(&depth).unwrap();
                self.results.insert((*contract, slot, lang));
            }
        }
        Ok(())
    }

    fn parse_sha3(&mut self, log: StructLog) -> Result<()> {
        let memory = hex::decode(log.memory.as_ref()
            .expect("SHA3 op should have memory content")
            .join("")
        )?;
        let stack = log.stack.as_ref()
            .expect("SHA3 op should have stack content");
        let mem_offset = stack[stack.len()-1].as_usize();
        let mem_length = stack[stack.len()-2].as_usize();
        if mem_length == 64 { // Only concerned about storage mappings
            let hashed_val = memory[mem_offset..mem_offset+mem_length].to_vec();
            let hash = H256(ethers::utils::keccak256(&hashed_val));
            let hashed_val_0: [u8; 32] = hashed_val[0..32].to_vec().try_into().unwrap();
            let hashed_val_1: [u8; 32] = hashed_val[32..64].to_vec().try_into().unwrap();
            self.hashed_vals.insert(hash, (H256(hashed_val_0), H256(hashed_val_1)));
        }
        Ok(())
    }

    fn parse_call(&mut self, log: StructLog, depth: usize) -> Result<()> {
        let stack = log.stack.expect("CALL op should have stack content");
        let address = c::u256_to_h160(stack[stack.len()-2]);
        self.depth_to_address.insert(depth + 1, address);
        Ok(())
    }

    fn parse_delegatecall(&mut self, depth: usize) -> Result<()> {
        let prev_address = *self.depth_to_address.get(&depth).unwrap();
        self.depth_to_address.insert(depth + 1, prev_address);
        Ok(())
    }

}
