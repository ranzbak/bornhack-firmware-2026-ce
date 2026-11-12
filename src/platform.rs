//! Firmware-vs-simulator shims shared by several modules: uptime, entropy,
//! the xorshift32 step, battery level and wall-clock seconds.  Keeps the
//! `cfg(embassy-core)` / `cfg(simulator)` split in one place.

/// Milliseconds of uptime: embassy clock on firmware, process uptime on the
/// simulator.
pub fn uptime_ms() -> u64 {
    #[cfg(feature = "embassy-core")]
    {
        embassy_time::Instant::now().as_millis()
    }
    #[cfg(not(feature = "embassy-core"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;
        static START: OnceLock<Instant> = OnceLock::new();
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_millis() as u64
    }
}

/// Low 32 bits of [`uptime_ms`].
pub fn uptime_ms_u32() -> u32 {
    uptime_ms() as u32
}

/// Pseudo-random byte from the clock: low byte of the embassy tick counter on
/// firmware, of the UNIX-epoch nanoseconds on the simulator.
pub fn entropy_byte() -> u8 {
    #[cfg(feature = "embassy-core")]
    {
        embassy_time::Instant::now().as_ticks() as u8
    }
    #[cfg(not(feature = "embassy-core"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u8)
            .unwrap_or(0xCB)
    }
}

/// Seed for a puzzle generator: low 32 bits of the embassy tick counter on
/// firmware, a fixed constant on the simulator (deterministic puzzles).
pub fn seed_ticks_or_fixed() -> u32 {
    #[cfg(feature = "embassy-core")]
    {
        embassy_time::Instant::now().as_ticks() as u32
    }
    #[cfg(not(feature = "embassy-core"))]
    {
        0xCAFE_BABE
    }
}

/// One xorshift32 step (13/17/5).
#[inline]
pub fn xorshift32(mut x: u32) -> u32 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

/// Battery level in percent; 100 on the simulator.
pub fn battery_pct() -> u8 {
    #[cfg(feature = "embassy-core")]
    {
        crate::fw::battery::read_pct()
    }
    #[cfg(not(feature = "embassy-core"))]
    {
        100
    }
}

/// `true` while the battery charger is active; always `false` on the simulator.
pub fn is_charging() -> bool {
    #[cfg(feature = "embassy-core")]
    {
        crate::fw::battery::is_charging()
    }
    #[cfg(not(feature = "embassy-core"))]
    {
        false
    }
}

/// UNIX seconds if the wall clock is known (always on the simulator).
pub fn unix_secs() -> Option<u32> {
    #[cfg(feature = "embassy-core")]
    {
        crate::unix_now()
    }
    #[cfg(not(feature = "embassy-core"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        Some(SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as u32)
    }
}
