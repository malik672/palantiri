#![feature(trivial_bounds)]


use serde::{Deserialize, Serialize};

pub mod checkpoint;
pub mod concensus;
pub mod libp2p;
pub mod light_client;

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


///NETWORKS:  [Mainnet. Holesky, Sepolia]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Fork {
    pub epoch: u64,
    pub fork_version: [u8; 4],
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkForks {
    pub genesis: Fork,
    pub altair: Fork,
    pub bellatrix: Fork,
    pub capella: Fork,
    pub deneb: Fork,
}

impl Default for NetworkForks {
    fn default() -> Self {
        Self {
            genesis: Fork {
                epoch: 0,
                fork_version: [0, 0, 0, 0],
            },
            altair: Fork {
                epoch: 0,
                fork_version: [0, 0, 0, 0],
            },
            bellatrix: Fork {
                epoch: 0,
                fork_version: [0, 0, 0, 0],
            },
            capella: Fork {
                epoch: 0,
                fork_version: [0, 0, 0, 0],
            },
            deneb: Fork {
                epoch: 0,
                fork_version: [0, 0, 0, 0],
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Holesky,
    Sepolia,
}

impl Network {
    pub fn default_forks(&self) -> NetworkForks {
        match self {
            Network::Mainnet => NetworkForks {
                genesis: Fork {
                    epoch: 0,
                    fork_version: [0, 0, 0, 0],
                },
                altair: Fork {
                    epoch: 74240,
                    fork_version: [1, 0, 0, 0],
                },
                bellatrix: Fork {
                    epoch: 144896,
                    fork_version: [2, 0, 0, 0],
                },
                capella: Fork {
                    epoch: 194048,
                    fork_version: [3, 0, 0, 0],
                },
                deneb: Fork {
                    epoch: 269568,
                    fork_version: [4, 0, 0, 0],
                },
            },
            Network::Holesky => NetworkForks {
                genesis: Fork {
                    epoch: 0,
                    fork_version: [1, 1, 7, 0],
                },
                altair: Fork {
                    epoch: 0,
                    fork_version: [2, 1, 7, 0],
                },
                bellatrix: Fork {
                    epoch: 0,
                    fork_version: [3, 1, 7, 0],
                },
                capella: Fork {
                    epoch: 256,
                    fork_version: [4, 1, 7, 0],
                },
                deneb: Fork {
                    epoch: 29696,
                    fork_version: [5, 1, 7, 0],
                },
            },
            Network::Sepolia => NetworkForks {
                genesis: Fork {
                    epoch: 0,
                    fork_version: [9, 0, 0, 105],
                },
                altair: Fork {
                    epoch: 50,
                    fork_version: [9, 0, 0, 106],
                },
                bellatrix: Fork {
                    epoch: 100,
                    fork_version: [9, 0, 0, 107],
                },
                capella: Fork {
                    epoch: 56832,
                    fork_version: [9, 0, 0, 108],
                },
                deneb: Fork {
                    epoch: 132608,
                    fork_version: [9, 0, 0, 109],
                },
            },
        }
    }
}



#[cfg(test)]

mod tests {
    #[test]
    pub fn test() {
        const FORK: u32 = 0b00001 |           // Genesis   (bits 0-4)
        0b00010 << 5  |     // Altair    (bits 5-9)
        0b00100 << 10 |     // Bellatrix (bits 10-14) 
        0b01000 << 15 |     // Capella   (bits 15-19)
        0b10000 << 20; // Deneb     (bits 20-24)
        println!("{:?}, {:?}", FORK, 0b01000 << 15);
    }
}