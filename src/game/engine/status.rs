//! The status word: the pet's worst problem in one word, plus whether it is
//! dangerous. It is the largest term of the mood penalty
//! ([`regime::penalty_terms`]); under 3 points nothing is shown (Content).

use super::GameState;
use super::needs::{self, DIRT_DANGER_PCT, HEALTH_DANGER_PCT, NEED_DANGER_LO_PCT, P};
use super::regime::{PENALTY_TERMS, penalty_terms};

/// Largest term must beat this to be named.
const CONTENT_UNDER: i32 = 3 * P;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
#[repr(u8)]
pub enum StatusWord {
    Content,
    Hungry,
    Stuffed,
    Thirsty,
    Waterlogged,
    Tired,
    Bored,
    Wired,
    Sloppy,
    Grubby,
    Overweight,
    Underweight,
    Unfit,
    HungOver,
    Injured,
    Groggy,
    Poorly,
}

impl StatusWord {
    /// Whether this word, even without danger, is worth a buzz. The
    /// "miserable" words still animate but stay quiet.
    pub fn alerts(self) -> bool {
        matches!(
            self,
            StatusWord::Hungry
                | StatusWord::Thirsty
                | StatusWord::Tired
                | StatusWord::Groggy
                | StatusWord::HungOver
                | StatusWord::Injured
                | StatusWord::Poorly
        )
    }
}

/// The words in [`penalty_terms`] order.
const WORDS: [StatusWord; PENALTY_TERMS] = [
    StatusWord::Hungry,
    StatusWord::Stuffed,
    StatusWord::Thirsty,
    StatusWord::Waterlogged,
    StatusWord::Tired,
    StatusWord::Bored,
    StatusWord::Wired,
    StatusWord::Sloppy,
    StatusWord::Grubby,
    StatusWord::Overweight,
    StatusWord::Underweight,
    StatusWord::Unfit,
    StatusWord::HungOver,
    StatusWord::Injured,
    StatusWord::Groggy,
    StatusWord::Poorly,
];

/// The status word and whether the pet is in danger (a need or the health
/// is past its danger line). A tie goes to the earlier word.
pub fn status(s: &GameState) -> (StatusWord, bool) {
    let mut word = StatusWord::Content;
    let mut best = CONTENT_UNDER;
    for (term, w) in penalty_terms(s).into_iter().zip(WORDS) {
        if term > best {
            best = term;
            word = w;
        }
    }
    let danger = (s.fullness as i32) < NEED_DANGER_LO_PCT as i32 * P
        || (s.hydration as i32) < NEED_DANGER_LO_PCT as i32 * P
        || s.booze as i32 > needs::booze_danger()
        || s.dirt as i32 > DIRT_DANGER_PCT as i32 * P
        || (s.health as i32) < HEALTH_DANGER_PCT as i32 * P;
    (word, danger)
}
