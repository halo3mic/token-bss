use redis::{Commands, Connection};
use alloy::primitives::Address;
use eyre::Result;
use crate::handlers::SearchResponse;
use crate::config::RedisConfig;
use crate::state::Chain;


pub struct RedisConnection(Connection);

impl RedisConnection {
    
    pub fn connect(RedisConfig { addr, password, is_tls }: RedisConfig) -> Result<Self> {
        println!("{:?}", (addr.clone(), password.clone(), is_tls));
        let password = password.unwrap_or_default();
        let uri_scheme = match is_tls {
            true => "rediss",
            false => "redis",
        };
        let redis_conn_url = format!("{uri_scheme}://:{password}@{addr}");
        let conn = redis::Client::open(redis_conn_url)?.get_connection()?;
        Ok(Self(conn))
    }

    pub fn store_entry(&mut self, address: &Address, chain_id: &Chain, result: &SearchResponse) -> Result<()> {
        let key = make_key(address, chain_id);
        let val_str = serde_json::to_string(result)?;
        self.0.set(&key, val_str)?;
        Ok(())
    }

    pub fn get_entry(&mut self, address: &Address, chain_id: &Chain) -> Result<Option<SearchResponse>> {
        let key = make_key(address, chain_id);
        let val_str: Option<String> = self.0.get(&key)?;
        let val = val_str.map(|val| serde_json::from_str(&val)).transpose()?;
        Ok(val)
    }

}

#[inline]
fn make_key(address: &Address, chain_id: &Chain) -> String {
    format!("{:?}:{}", address, chain_id)
}
