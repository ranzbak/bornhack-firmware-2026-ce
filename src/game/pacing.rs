//! Display pacing: how often a sprite animation advances a frame, and how
//! long the display loop sleeps before the next redraw.
//!
//! This is display pacing only. The engine still advances solely through
//! `update()` at its own wake ticks (the delta engine); nothing here steps or
//! polls it. An idle pet shows one frame per [`IDLE_FRAME_SECS`] to spare the
//! battery and the e-paper panel; action animations still play at full speed.

use super::engine::DisplayAnim;

/// Seconds in one engine tick.
pub const SECS_PER_TICK: u64 = 10;
/// One idle frame every 5 minutes (spec 17 row 29).
pub const IDLE_FRAME_SECS: u64 = 300;
/// The idle frame in engine ticks, derived so the two can never drift apart.
pub const IDLE_FRAME_TICKS: u32 = (IDLE_FRAME_SECS / SECS_PER_TICK) as u32;
/// Shortest time between two frames of a fast animation.
pub const MIN_FRAME_SECS: u64 = 2;

/// Hatching and the action animations are "fast"; everything else (Idle,
/// Happy, Warning*, Critical*, Sleeping, Leaving, Gone, Hibernating) is idle.
pub fn is_action_anim(anim: DisplayAnim) -> bool {
    matches!(
        anim,
        DisplayAnim::Feeding
            | DisplayAnim::Healing
            | DisplayAnim::Playing
            | DisplayAnim::Exercising
            | DisplayAnim::Medicating
            | DisplayAnim::Drinking
            | DisplayAnim::Ozempic
            | DisplayAnim::Rehab
            | DisplayAnim::OnlyPets
    )
}

/// Seconds per frame, `None` for a single-frame animation. Hatching spreads
/// its remaining ticks over the frames, action animations spread
/// `action_ticks_remaining`; both are floored at [`MIN_FRAME_SECS`]. Every
/// other animation is idle: [`IDLE_FRAME_SECS`].
pub fn frame_interval_secs(
    anim: DisplayAnim,
    frames: u32,
    action_ticks_remaining: u8,
) -> Option<u64> {
    if frames <= 1 {
        return None;
    }
    let frames = frames as u64;
    let spread = |ticks: u64| (ticks * SECS_PER_TICK / frames).max(MIN_FRAME_SECS);
    match anim {
        DisplayAnim::Hatching { ticks_remaining } => Some(spread(ticks_remaining as u64)),
        a if is_action_anim(a) => Some(spread(action_ticks_remaining as u64)),
        _ => Some(IDLE_FRAME_SECS),
    }
}

/// How long to wait, and whether the wait ends on a frame (`true`) or on an
/// engine/UI wake (`false`: redraw without advancing the frame).
pub fn sprite_wait(frame_secs: Option<u64>, wake_secs: u64) -> (u64, bool) {
    match frame_secs {
        Some(f) if f <= wake_secs => (f, true),
        _ => (wake_secs, false),
    }
}

/// Seconds until the next wake the display must honour: the engine's
/// `next_wake_tick` or the UI deadline, whichever is first, at least one tick
/// away. Saturating, because a hibernating pet's wake is `u32::MAX`.
pub fn wake_secs(next_wake_tick: u32, ui_deadline_tick: u32, now_tick: u32) -> u64 {
    let ticks = next_wake_tick
        .min(ui_deadline_tick)
        .saturating_sub(now_tick)
        .max(1);
    ticks as u64 * SECS_PER_TICK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_frame_is_five_minutes_actions_stay_fast() {
        assert_eq!(frame_interval_secs(DisplayAnim::Idle, 3, 0), Some(300));
        assert_eq!(frame_interval_secs(DisplayAnim::Sleeping, 4, 0), Some(300));
        assert_eq!(
            frame_interval_secs(DisplayAnim::WarningHungry, 2, 0),
            Some(300)
        );
        assert_eq!(frame_interval_secs(DisplayAnim::Exercising, 3, 6), Some(20));
        assert_eq!(frame_interval_secs(DisplayAnim::Feeding, 5, 1), Some(2));
        assert_eq!(
            frame_interval_secs(
                DisplayAnim::Hatching {
                    ticks_remaining: 12
                },
                4,
                0
            ),
            Some(30)
        );
        assert_eq!(frame_interval_secs(DisplayAnim::Idle, 1, 0), None);
    }

    #[test]
    fn sprite_wait_ends_at_the_engine_wake() {
        assert_eq!(sprite_wait(Some(300), 120), (120, false));
        assert_eq!(sprite_wait(Some(300), 300), (300, true));
        assert_eq!(sprite_wait(Some(300), 7200), (300, true));
        assert_eq!(sprite_wait(None, 50), (50, false));
    }

    #[test]
    fn wake_secs_never_underflows_or_overflows() {
        assert_eq!(wake_secs(105, u32::MAX, 100), 50);
        assert_eq!(wake_secs(90, u32::MAX, 100), 10, "a past wake is one tick");
        assert_eq!(wake_secs(u32::MAX, u32::MAX, 0), u32::MAX as u64 * 10);
        assert_eq!(
            wake_secs(500, 120, 100),
            200,
            "the UI deadline can come first"
        );
    }

    #[test]
    fn idle_frame_constants_agree() {
        assert_eq!(IDLE_FRAME_TICKS as u64 * SECS_PER_TICK, IDLE_FRAME_SECS);
    }

    #[test]
    fn only_actions_and_hatching_are_fast() {
        assert!(is_action_anim(DisplayAnim::Rehab));
        assert!(is_action_anim(DisplayAnim::OnlyPets));
        assert!(!is_action_anim(DisplayAnim::Sleeping));
        assert!(!is_action_anim(DisplayAnim::Hibernating));
        assert!(!is_action_anim(DisplayAnim::Idle));
    }
}
