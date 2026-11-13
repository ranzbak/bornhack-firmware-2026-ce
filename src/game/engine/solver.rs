//! Segment solver: how many minutes `advance` may take in one step.

use super::GameState;

/// Longest segment (at most `max` minutes) over which no threshold is crossed.
///
/// Stub: always one minute.  Task 7 replaces the body.
pub fn seg_len(_s: &GameState, max: u32) -> u32 {
    max.min(1)
}
