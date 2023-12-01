use fuel_types::Address;
use rand::Rng;
use std::collections::HashMap;

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Salt([u8; 32]);

impl Salt {
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes[..]);
        Salt(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub type Pow = (Address, Salt, u64);

pub type SessionMap = HashMap<Salt, Address>;
pub type ProofMap = HashMap<Pow, bool>;
