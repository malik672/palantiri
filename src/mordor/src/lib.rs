use chrono::{DateTime, TimeZone, Utc};
use std::time::Duration;

/// Default genesis timestamp for the Ethereum beacon chain (2020-12-01 12:00:23 UTC)
pub const DEFAULT_GENESIS_TIMESTAMP: i64 = 1606824023;
/// Duration of each slot in seconds
pub const SLOT_DURATION: u64 = 12;

/// Represents the current state of a slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotInfo {
    /// Current slot number
    pub slot: u64,
    /// Seconds elapsed in the current slot
    pub seconds_elapsed: u64,
    /// Seconds remaining in the current slot
    pub seconds_remaining: u64,
}

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

    pub fn current_slot(&self) -> Result<u64, SlotError> {
        let now = Utc::now();
        let duration_since_genesis = now
            .signed_duration_since(self.genesis_time)
            .num_seconds();
        
        if duration_since_genesis < 0 {
            return Err(SlotError::TimeCalculationError);
        }

        Ok(duration_since_genesis as u64 / SLOT_DURATION)
    }

    pub fn slot_info(&self) -> Result<SlotInfo, SlotError> {
        let now = Utc::now();
        let duration_since_genesis = now
            .signed_duration_since(self.genesis_time)
            .num_seconds();

        if duration_since_genesis < 0 {
            return Err(SlotError::TimeCalculationError);
        }

        let duration_secs = duration_since_genesis as u64;
        let slot = duration_secs / SLOT_DURATION;
        let seconds_elapsed = duration_secs % SLOT_DURATION;
        let seconds_remaining = SLOT_DURATION - seconds_elapsed;

        Ok(SlotInfo {
            slot,
            seconds_elapsed,
            seconds_remaining,
        })
    }

    pub fn time_until_next_slot(&self) -> Result<Duration, SlotError> {
        let info = self.slot_info()?;
        Ok(Duration::from_secs(info.seconds_remaining))
    }

    pub fn slot_timestamp(&self, slot: u64) -> DateTime<Utc> {
        self.genesis_time + chrono::Duration::seconds((slot * SLOT_DURATION) as i64)
    }

    pub fn slot_at_timestamp(&self, timestamp: DateTime<Utc>) -> Result<u64, SlotError> {
        let duration_since_genesis = timestamp
            .signed_duration_since(self.genesis_time)
            .num_seconds();
        
        if duration_since_genesis < 0 {
            return Err(SlotError::TimeCalculationError);
        }

        Ok(duration_since_genesis as u64 / SLOT_DURATION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_default_initialization() {
        let sync = SlotSynchronizer::default();
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
    fn test_slot_info() {
        let sync = SlotSynchronizer::default();
        let info = sync.slot_info().unwrap();
        assert!(info.seconds_elapsed < SLOT_DURATION);
        assert!(info.seconds_remaining <= SLOT_DURATION);
        assert_eq!(info.seconds_elapsed + info.seconds_remaining, SLOT_DURATION);
    }

    #[test]
    fn test_slot_timestamp() {
        let sync = SlotSynchronizer::default();
        let slot = 1000u64;
        let timestamp = sync.slot_timestamp(slot);
        let calculated_slot = sync.slot_at_timestamp(timestamp).unwrap();
        assert_eq!(slot, calculated_slot);
    }

    #[test]
    fn test_invalid_genesis() {
        assert!(SlotSynchronizer::new(i64::MIN).is_err());
    }

    #[test]
    fn test_future_slot_calculation() {
        let sync = SlotSynchronizer::default();
        let future_time = Utc::now() + Duration::days(365);
        assert!(sync.slot_at_timestamp(future_time).is_ok());
    }
}