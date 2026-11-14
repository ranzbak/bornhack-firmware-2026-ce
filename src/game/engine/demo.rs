//! Demo mode state and its save block (plan 3). The pet looks after itself
//! through the delta engine's events; this file holds only the saved state.
//! The block sits in the reserved tail of the format-3 save, so a plan-2
//! save (zeros there) reads as demo off.

/// Where the demo block starts in the save, right after `saved_unix`.
pub const BLOCK_AT: usize = 149;
/// Bytes in the demo block.
pub const BLOCK_LEN: usize = 11;

/// Minutes in a day: bedtime and getting-up lines count minutes since 00:00.
pub const MINUTES_PER_DAY: u16 = 1440;
/// Bits the temperament takes in the flags byte.
pub const LAZY_BITS: u8 = 4;
/// Largest temperament the flags byte holds.
pub const LAZY_MAX: u8 = (1 << LAZY_BITS) - 1;
/// Flags byte: where the temperament starts.
const LAZY_SHIFT: u8 = 8 - LAZY_BITS;
/// Bit 15 of the last field is `pet_action`; the hours use the low 15 bits.
const PET_ACTION_BIT: u16 = 1 << 15;
/// Most whole hours the saved lifetime holds (3.7 years).
const HOURS_MAX: u16 = PET_ACTION_BIT - 1;
const MIN_PER_HOUR: u32 = 60;

/// The saved demo block (save bytes 149..160). All zero = demo off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Demo {
    pub on: bool,
    /// Soft delay `next_min` is running.
    pub soft_armed: bool,
    /// `clock_min` counts to bedtime (awake) or getting up (asleep).
    pub clock_armed: bool,
    /// Temperament 0..=`DEMO_LAZY_MAX_PCT`, re-rolled every `DEMO_LAZY_MIN`.
    pub lazy: u8,
    pub clock_min: u16,
    pub whim_min: u16,
    pub lazy_min: u16,
    /// Soft delay or dry-out interval.
    pub next_min: u8,
    /// Policy paused after the player acted.
    pub pause_min: u8,
    /// Care mode: false = instant with a note (default), true = animated.
    pub animated: bool,
    /// The running action is the pet's own.
    pub pet_action: bool,
    /// Minutes lived in demo; saved as whole hours (15 bits).
    pub demo_min: u32,
}

impl Demo {
    pub fn write(&self, b: &mut [u8; BLOCK_LEN]) {
        b[0] = self.on as u8
            | (self.soft_armed as u8) << 1
            | (self.clock_armed as u8) << 2
            | (self.animated as u8) << 3
            | (self.lazy.min(LAZY_MAX)) << LAZY_SHIFT;
        b[1..3].copy_from_slice(&self.clock_min.to_le_bytes());
        b[3..5].copy_from_slice(&self.whim_min.to_le_bytes());
        b[5..7].copy_from_slice(&self.lazy_min.to_le_bytes());
        b[7] = self.next_min;
        b[8] = self.pause_min;
        let hours = (self.demo_min / MIN_PER_HOUR).min(HOURS_MAX as u32) as u16;
        let last = hours | if self.pet_action { PET_ACTION_BIT } else { 0 };
        b[9..11].copy_from_slice(&last.to_le_bytes());
    }

    pub fn read(b: &[u8; BLOCK_LEN]) -> Demo {
        let last = u16::from_le_bytes([b[9], b[10]]);
        Demo {
            on: b[0] & 1 != 0,
            soft_armed: b[0] & 2 != 0,
            clock_armed: b[0] & 4 != 0,
            animated: b[0] & 8 != 0,
            lazy: b[0] >> LAZY_SHIFT,
            clock_min: u16::from_le_bytes([b[1], b[2]]),
            whim_min: u16::from_le_bytes([b[3], b[4]]),
            lazy_min: u16::from_le_bytes([b[5], b[6]]),
            next_min: b[7],
            pause_min: b[8],
            pet_action: last & PET_ACTION_BIT != 0,
            demo_min: (last & HOURS_MAX) as u32 * MIN_PER_HOUR,
        }
    }
}

/// The demo pet's dice state. Transient (never saved), so two states that
/// differ only in their dice compare equal: a save round trip must not
/// change equality.
#[derive(Clone, Copy, Debug)]
pub struct Dice(pub u32);

impl PartialEq for Dice {
    fn eq(&self, _: &Dice) -> bool {
        true
    }
}

/// A nonzero rng state from two numbers (xorshift32 would stick at 0).
pub fn seed_rng(a: u64, b: u64) -> Dice {
    let mixed = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    let folded = (mixed ^ (mixed >> 32)) as u32;
    Dice(if folded == 0 { 1 } else { folded })
}
