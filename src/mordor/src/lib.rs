use chrono::{DateTime, TimeZone, Utc};
use std::time::Duration;

/// Default genesis timestamp for the Ethereum beacon chain (2020-12-01 12:00:23 UTC)
pub const DEFAULT_GENESIS_TIMESTAMP: i64 = 1606824023;
/// Duration of each slot in seconds
pub const SLOT_DURATION: u64 = 12;

pub type SlotTiming = (u64, u8, u8);

/// Error types for the slot synchronizer
#[derive(Debug, thiserror::Error)]
pub enum SlotError {
    #[error("Invalid genesis time")]
    InvalidGenesisTime,
    #[error("Time calculation error")]
    TimeCalculationError,
}

#[derive(Debug, Clone)]
pub struct SlotSynchronizer {
    genesis_time: DateTime<Utc>,
}

impl Default for SlotSynchronizer {
    fn default() -> Self {
        Self::new(DEFAULT_GENESIS_TIMESTAMP).expect("Default genesis timestamp should be valid")
    }
}

impl SlotSynchronizer {
    pub fn new(genesis_timestamp: i64) -> Result<Self, SlotError> {
        let genesis_time = Utc
            .timestamp_opt(genesis_timestamp, 0)
            .single()
            .ok_or(SlotError::InvalidGenesisTime)?;

        Ok(Self { genesis_time })
    }

    pub fn current_slot(&self) -> Result<u64, ()> {
        let now = Utc::now();
        let duration_since_genesis = now
            .signed_duration_since(self.genesis_time)
            .num_seconds();
        

        Ok(duration_since_genesis as u64 / SLOT_DURATION)
    }

    ///Calulates the nearest slot to the current time
    /// Returns slot number, seconds remaining used for the current slot and seconds remaining for the next slot
    pub fn slot_info(&self) -> Result<SlotTiming, ()> {
        let now = Utc::now();
        let duration_since_genesis = now
            .signed_duration_since(self.genesis_time)
            .num_seconds();


        let duration_secs = duration_since_genesis as u64;
        let slot = duration_secs / SLOT_DURATION;
        let seconds_elapsed = duration_secs % SLOT_DURATION;
        let seconds_remaining = SLOT_DURATION - seconds_elapsed;

        Ok((slot, seconds_elapsed as u8, seconds_remaining as u8))
    }

    pub fn time_until_next_slot(&self) -> Result<Duration, SlotError> {
        let info = self.slot_info().unwrap();
        Ok(Duration::from_secs(info.2 as u64))
    }

    pub fn slot_timestamp(&self, slot: u64) -> DateTime<Utc> {
        self.genesis_time + chrono::Duration::seconds((slot * SLOT_DURATION) as i64)
    }

    pub fn slot_at_timestamp(&self, timestamp: DateTime<Utc>) -> u64 {
        let duration_since_genesis = timestamp
            .signed_duration_since(self.genesis_time)
            .num_seconds();
        

        duration_since_genesis as u64 / SLOT_DURATION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_initialization() {
        let sync = SlotSynchronizer::default();
        println!("{:?}", sync.slot_info().unwrap());
        assert!(sync.current_slot().is_ok());
    }

    #[test]
    fn test_custom_genesis() {
        let now = Utc::now().timestamp();
        let sync = SlotSynchronizer::new(now).unwrap();
        let slot = sync.current_slot().unwrap();
        assert_eq!(slot, 0);
    }

    #[test]
    fn test_slot_timestamp() {
        let sync = SlotSynchronizer::default();
        let slot = 1000u64;
        let timestamp = sync.slot_timestamp(slot);
        let calculated_slot = sync.slot_at_timestamp(timestamp);
        assert_eq!(slot, calculated_slot);
    }

    #[test]
    fn test_invalid_genesis() {
        assert!(SlotSynchronizer::new(i64::MIN).is_err());
    }

}