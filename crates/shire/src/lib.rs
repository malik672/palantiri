#![feature(trivial_bounds)]

pub mod concensus;
pub mod concensus_rpc;
pub mod libp2p;

// Single cache-line optimized fork bitmap ||  5 bits per fork ||
const FORK_BITS: u32 = 0b11111;
const FORK: u32 = 0b00001 |           // Genesis   (bits 0-4)
    0b00010 << 5  |     // Altair    (bits 5-9)
    0b00100 << 10 |     // Bellatrix (bits 10-14) 
    0b01000 << 15 |     // Capella   (bits 15-19)
    0b10000 << 20; // Deneb     (bits 20-24)

#[derive(Debug, Clone, Copy)]
pub struct Forks(u32);

impl Default for Forks {
    fn default() -> Self {
        Self::new()
    }
}

impl Forks {
    pub fn new() -> Self {
        Self(FORK)
    }

    pub fn with_forks(mut self, forks: u32) -> Self {
        self.0 |= forks;
        self
    }

    pub fn is_genesis(&self) -> bool {
        (self.0 & FORK_BITS) != 0
    }

    pub fn is_altair(&self) -> bool {
        self.0 & (FORK_BITS << 5) != 0
    }

    pub fn is_bellatrix(&self) -> bool {
        self.0 & (FORK_BITS << 10) != 0
    }

    pub fn is_capella(&self) -> bool {
        self.0 & (FORK_BITS << 15) != 0
    }

    pub fn is_deneb(&self) -> bool {
        self.0 & (FORK_BITS << 20) != 0
    }
}
