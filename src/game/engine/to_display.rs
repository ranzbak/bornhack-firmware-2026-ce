//! Display animation state — determines what the pet area should show.
//!
//! [`DisplayAnim`] is a simple enum that the display layer matches on to
//! pick the right animation.  [`GameState::display_anim()`] is a cheap
//! read-only lookup — no update, no stats computation.  Call it every
//! display refresh without worry.
//!
//! Priority (highest wins):
//!   1. Gone (pet has left)
//!   2. Hibernating
//!   3. Hatching (egg animation)
//!   4. Active action (feeding, healing, relaxing, playing, sleeping)
//!   5. Leaving (countdown warning, with urgency level)
//!   6. Status word in danger (urgent distress — needs immediate action)
//!   7. Status word (attention needed soon — not yet in danger)
//!   8. Idle / Happy (content; happy from 80 % mood)

use super::needs::{HAPPY_MOOD_PCT, P};
use super::status::{StatusWord, status};
use super::{Action, GameState, Phase};


/// What the pet display area should show right now.
///
/// The display layer matches on this and selects the appropriate
/// animation / sprite sequence.  Variants are ordered by priority —
/// only the highest-priority active state is returned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum DisplayAnim {
    // ── Group 1: terminal / blocking ────────────────────────────────
    /// Pet has left permanently.
    Gone,
    /// All progression frozen.
    Hibernating,
    /// Egg is hatching.  `ticks_remaining` counts down to zero.
    Hatching {
        ticks_remaining: u16,
    },

    // ── Group 2: active actions (mutually exclusive) ────────────────
    Feeding,
    Healing,
    Playing,
    Sleeping,
    Exercising,
    Medicating,
    /// Drinking a beverage (Beer / Wine / Whiskey / Water / Cola).
    Drinking,
    /// Receiving an Ozempic injection.
    Ozempic,
    /// Rehab / sobering-up treatment.
    Rehab,
    /// "Only pets" work/hobby action.
    OnlyPets,

    // ── Group 3: leaving danger ─────────────────────────────────────
    /// Pet is about to leave.  `maxed_count` (1–4) indicates urgency.
    Leaving {
        maxed_count: u8,
    },

    // ── Group 4: critical stats (needs immediate action) ────────────
    CriticalSick,
    CriticalTired,
    CriticalHungry,

    // ── Group 5: warning stats (attention needed soon) ──────────────
    WarningSick,
    WarningTired,
    WarningHungry,
    WarningMiserable,

    // ── Group 6: content ────────────────────────────────────────────
    /// Pet is happy (all stats well below warning thresholds).
    Happy,
    /// Default resting state (no warnings, no special happiness).
    Idle,
}

/// Count of needs at level 0.
fn count_maxed(state: &GameState) -> u8 {
    (state.fullness == 0) as u8 + (state.energy == 0) as u8 + (state.health == 0) as u8
}

impl GameState {
    /// Determine which animation the display area should show.
    ///
    /// This is a **cheap read-only lookup** — no update is triggered.
    /// Call it from the display loop on every refresh.
    pub fn display_anim(&self) -> DisplayAnim {
        // ── Group 1: terminal / blocking ────────────────────────────
        if self.phase == Phase::Gone {
            return DisplayAnim::Gone;
        }
        if self.hibernating {
            return DisplayAnim::Hibernating;
        }
        if self.phase == Phase::Hatching {
            return DisplayAnim::Hatching {
                ticks_remaining: self.hatching_countdown,
            };
        }

        // ── Group 2: active action ──────────────────────────────────
        if let Some(action) = self.active_action {
            return match action {
                Action::Feed => DisplayAnim::Feeding,
                Action::Heal => DisplayAnim::Healing,
                Action::Play => DisplayAnim::Playing,
                Action::Exercise => DisplayAnim::Exercising,
                Action::Medicate => DisplayAnim::Medicating,
                Action::Ozempic => DisplayAnim::Ozempic,
                Action::Rehab => DisplayAnim::Rehab,
                Action::Drink => DisplayAnim::Drinking,
                Action::OnlyPets => DisplayAnim::OnlyPets,
                // Wash and Relax have no sprites of their own; they show the play animation.
                Action::Wash | Action::Relax => DisplayAnim::Playing,
            };
        }
        if self.is_sleeping {
            return DisplayAnim::Sleeping;
        }

        // ── Group 3: leaving danger ─────────────────────────────────
        if self.phase == Phase::Leaving {
            return DisplayAnim::Leaving {
                maxed_count: count_maxed(self),
            };
        }

        // Note: diabetic-and-unmedicated is deliberately *not* a
        // DisplayAnim state — there's no sprite art for it (or for the
        // weight critical/warning tiers below), and swapping the pet's
        // whole display out for a debug-text-only screen made the pet
        // disappear entirely whenever the condition was active. It's
        // surfaced instead as a persistent on-screen banner drawn
        // alongside whatever the pet's normal animation is — see
        // `lifecycle::is_diabetic_unmedicated()` / `game::mod` render.

        // ── Groups 4 to 6: the status word ──────────────────────────
        let (word, danger) = status(self);
        let pick = |warning, critical| if danger { critical } else { warning };
        match word {
            StatusWord::Hungry | StatusWord::Thirsty => {
                pick(DisplayAnim::WarningHungry, DisplayAnim::CriticalHungry)
            }
            StatusWord::Tired | StatusWord::Groggy | StatusWord::HungOver => {
                pick(DisplayAnim::WarningTired, DisplayAnim::CriticalTired)
            }
            StatusWord::Poorly | StatusWord::Injured => {
                pick(DisplayAnim::WarningSick, DisplayAnim::CriticalSick)
            }
            StatusWord::Bored
            | StatusWord::Wired
            | StatusWord::Sloppy
            | StatusWord::Grubby
            | StatusWord::Stuffed
            | StatusWord::Waterlogged
            | StatusWord::Overweight
            | StatusWord::Underweight
            | StatusWord::Unfit => DisplayAnim::WarningMiserable,
            StatusWord::Content if self.mood as i32 >= HAPPY_MOOD_PCT as i32 * P => DisplayAnim::Happy,
            StatusWord::Content => DisplayAnim::Idle,
        }
    }
}
