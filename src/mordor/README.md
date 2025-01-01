# Mordor 👁

The Eye of Sauron for Ethereum Beacon Chain slot synchronization.

## The Dark Tower's Purpose

Mordor watches over the Ethereum beacon chain's slots with unwavering vigilance, providing precise timing calculations that would make even the Dark Lord proud. Never miss a slot again with the all-seeing eye of slot synchronization.

## Powers Granted by the One Ring

- Gaze into the current beacon chain slot
- Peer into the depths of slot timing (elapsed/remaining time)
- Command the conversion between slots and timestamps
- Dominate slot boundary calculations
- Rule over the genesis time of the beacon chain
- Guard against timing miscalculations

## Summoning the Dark Powers

```rust
use mordor::SlotSynchronizer;

// Forge the ring with default genesis time
let eye = SlotSynchronizer::default();

// Gaze into current slot
let vision = eye.slot_info().unwrap();
println!("The eye sees slot: {}", vision.slot);
println!("Time elapsed in the slot: {}", vision.seconds_elapsed);
println!("Time remaining until next slot: {}", vision.seconds_remaining);
```

## Dark Arts Technical Specifications

- Genesis timestamp: The First Age of December 1, 2020, 12:00:23 UTC (1606824023)
- Each slot lasts: 12 seconds in the mortal realm
- All calculations are performed in UTC, the one true timezone
- Powerful error handling to prevent the eye from being deceived

## Forging Your Own Ring

Add this to your `Cargo.toml`:

```toml
[dependencies]
mordor = "0.1.0"
```

## The Black Gate (License)

MIT License - Free as the darkness itself

## Join the Army of Mordor

Pull requests are welcome. The eye sees all contributions.

*One Ring to sync them all, One Ring to time them,  
One Ring to watch them all and in the darkness bind them*