//! The status word: the pet's worst problem in one word, plus whether it is
//! dangerous. It is the largest term of the mood penalty
//! ([`regime::penalty_terms`]); under 3 points nothing is shown (Content).

use heapless::String;

use super::GameState;
use super::needs::{
    self, DIRT_DANGER_PCT, EXHAUSTED_PCT, HEALTH_DANGER_PCT, HUNGRY_PCT, NEED_DANGER_LO_PCT, P,
    SORE_PCT, TIPSY_PCT,
};
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

/// The first danger word, in the order fullness, hydration, booze, dirt,
/// health. `status` and the status line both read it, so the danger lines
/// exist once.
pub fn danger_word(s: &GameState) -> Option<Word> {
    if (s.fullness as i32) < NEED_DANGER_LO_PCT as i32 * P {
        Some(Word::Starving)
    } else if (s.hydration as i32) < NEED_DANGER_LO_PCT as i32 * P {
        Some(Word::Parched)
    } else if s.booze as i32 > needs::booze_danger() {
        Some(Word::Wasted)
    } else if s.dirt as i32 > DIRT_DANGER_PCT as i32 * P {
        Some(Word::Filthy)
    } else if (s.health as i32) < HEALTH_DANGER_PCT as i32 * P {
        Some(Word::Poorly)
    } else {
        None
    }
}

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
    (word, danger_word(s).is_some())
}

/// A second word must beat this penalty to be shown beside the first.
pub const SECOND_WORD_UNDER: i32 = 10 * P;

/// Widest line of the status line, in columns (7 px font on a 152 px panel).
const LINE_COLS: usize = 21;

/// Everything the status line can say: graded problems, prompts and info.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
#[repr(u8)]
pub enum Word {
    Peckish,
    Hungry,
    Starving,
    Stuffed,
    Thirsty,
    Parched,
    Waterlogged,
    Sleepy,
    Exhausted,
    Bored,
    Wired,
    Tipsy,
    Sloppy,
    Wasted,
    Grubby,
    Filthy,
    Overweight,
    Underweight,
    Unfit,
    HungOver,
    Sore,
    Injured,
    Groggy,
    Poorly,
    NoInsulin,
    NeedsRehab,
    Asleep,
    WellRested,
    Content,
}

impl Word {
    pub const ALL: [Word; 29] = [
        Word::Peckish,
        Word::Hungry,
        Word::Starving,
        Word::Stuffed,
        Word::Thirsty,
        Word::Parched,
        Word::Waterlogged,
        Word::Sleepy,
        Word::Exhausted,
        Word::Bored,
        Word::Wired,
        Word::Tipsy,
        Word::Sloppy,
        Word::Wasted,
        Word::Grubby,
        Word::Filthy,
        Word::Overweight,
        Word::Underweight,
        Word::Unfit,
        Word::HungOver,
        Word::Sore,
        Word::Injured,
        Word::Groggy,
        Word::Poorly,
        Word::NoInsulin,
        Word::NeedsRehab,
        Word::Asleep,
        Word::WellRested,
        Word::Content,
    ];

    pub fn text(self) -> &'static str {
        match self {
            Word::Peckish => "peckish",
            Word::Hungry => "hungry",
            Word::Starving => "starving",
            Word::Stuffed => "stuffed",
            Word::Thirsty => "thirsty",
            Word::Parched => "parched",
            Word::Waterlogged => "waterlogged",
            Word::Sleepy => "sleepy",
            Word::Exhausted => "exhausted",
            Word::Bored => "bored",
            Word::Wired => "wired",
            Word::Tipsy => "tipsy",
            Word::Sloppy => "sloppy",
            Word::Wasted => "wasted",
            Word::Grubby => "grubby",
            Word::Filthy => "filthy",
            Word::Overweight => "overweight",
            Word::Underweight => "underweight",
            Word::Unfit => "unfit",
            Word::HungOver => "hung over",
            Word::Sore => "sore",
            Word::Injured => "injured",
            Word::Groggy => "groggy",
            Word::Poorly => "poorly",
            Word::NoInsulin => "no insulin",
            Word::NeedsRehab => "needs rehab",
            Word::Asleep => "asleep",
            Word::WellRested => "well rested",
            Word::Content => "content",
        }
    }
}

/// The graded word of a penalty term: the same problem, worse when it
/// crosses a fixed edge.
fn graded(s: &GameState, w: StatusWord) -> Word {
    match w {
        StatusWord::Hungry if (s.fullness as i32) < NEED_DANGER_LO_PCT as i32 * P => Word::Starving,
        StatusWord::Hungry if (s.fullness as i32) < HUNGRY_PCT as i32 * P => Word::Hungry,
        StatusWord::Hungry => Word::Peckish,
        StatusWord::Thirsty if (s.hydration as i32) < NEED_DANGER_LO_PCT as i32 * P => {
            Word::Parched
        }
        StatusWord::Thirsty => Word::Thirsty,
        StatusWord::Tired if (s.energy as i32) < EXHAUSTED_PCT as i32 * P => Word::Exhausted,
        StatusWord::Tired => Word::Sleepy,
        StatusWord::Sloppy if s.booze as i32 > needs::booze_danger() => Word::Wasted,
        StatusWord::Sloppy => Word::Sloppy,
        StatusWord::Grubby if s.dirt as i32 > DIRT_DANGER_PCT as i32 * P => Word::Filthy,
        StatusWord::Grubby => Word::Grubby,
        StatusWord::Stuffed => Word::Stuffed,
        StatusWord::Waterlogged => Word::Waterlogged,
        StatusWord::Bored => Word::Bored,
        StatusWord::Wired => Word::Wired,
        StatusWord::Overweight => Word::Overweight,
        StatusWord::Underweight => Word::Underweight,
        StatusWord::Unfit => Word::Unfit,
        StatusWord::HungOver => Word::HungOver,
        StatusWord::Injured => Word::Injured,
        StatusWord::Groggy => Word::Groggy,
        StatusWord::Poorly => Word::Poorly,
        StatusWord::Content => Word::Content,
    }
}

/// The one or two words under the pet, and whether the pet is in danger.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub struct StatusLine {
    pub first: Word,
    pub second: Option<Word>,
    pub danger: bool,
}

impl StatusLine {
    /// "a, b" when it fits one line, else "a\nb".
    pub fn text(&self) -> String<32> {
        let mut out = String::new();
        let a = self.first.text();
        let _ = out.push_str(a);
        if let Some(b) = self.second {
            let b = b.text();
            let sep = if a.len() + 2 + b.len() <= LINE_COLS {
                ", "
            } else {
                "\n"
            };
            let _ = out.push_str(sep);
            let _ = out.push_str(b);
        }
        out
    }
}

/// The status line: the danger word, the worst graded problem, a prompt, a
/// second problem (when big enough), else an info word. Every threshold it
/// reads is a fixed edge, a `solver::sig` line or a timer bound, so the wake
/// stays exact.
pub fn status_line(s: &GameState) -> StatusLine {
    let terms = penalty_terms(s);
    // Largest term above `above`, earlier wins a tie, skipping `skip`.
    let largest = |above: i32, skip: Option<usize>| {
        let mut best = None;
        let mut value = above;
        for (i, &t) in terms.iter().enumerate() {
            if Some(i) != skip && t > value {
                value = t;
                best = Some(i);
            }
        }
        best
    };
    let top = largest(CONTENT_UNDER, None);
    let next = top.and_then(|t| largest(SECOND_WORD_UNDER, Some(t)));
    let prompt = if s.is_diabetic_unmedicated() {
        Some(Word::NoInsulin)
    } else if s.is_alcoholic_untreated() {
        Some(Word::NeedsRehab)
    } else {
        None
    };
    let mut first = None;
    let mut second = None;
    let candidates = [
        danger_word(s),
        top.map(|i| graded(s, WORDS[i])),
        prompt,
        next.map(|i| graded(s, WORDS[i])),
    ];
    for w in candidates.into_iter().flatten() {
        if first == Some(w) {
            continue;
        }
        if first.is_none() {
            first = Some(w);
        } else if second.is_none() {
            second = Some(w);
        }
    }
    let first = first.unwrap_or_else(|| info_word(s));
    StatusLine {
        first,
        second,
        danger: danger_word(s).is_some(),
    }
}

/// What to say when nothing is out of line.
fn info_word(s: &GameState) -> Word {
    if s.is_sleeping {
        Word::Asleep
    } else if conditions(s).has(Condition::Sore) {
        Word::Sore
    } else if s.booze as i32 >= TIPSY_PCT as i32 * P {
        Word::Tipsy
    } else if s.rested_min > 0 {
        Word::WellRested
    } else {
        Word::Content
    }
}

/// Lasting states shown on the stats view, most pressing first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
#[repr(u8)]
pub enum Condition {
    Injured,
    Sore,
    HungOver,
    Groggy,
    WornOut,
    Diabetic,
    Dependent,
    InRehab,
    WellRested,
}

impl Condition {
    pub const ALL: [Condition; 9] = [
        Condition::Injured,
        Condition::Sore,
        Condition::HungOver,
        Condition::Groggy,
        Condition::WornOut,
        Condition::Diabetic,
        Condition::Dependent,
        Condition::InRehab,
        Condition::WellRested,
    ];

    pub fn text(self) -> &'static str {
        match self {
            Condition::Injured => "injured",
            Condition::Sore => "sore",
            Condition::HungOver => "hung over",
            Condition::Groggy => "groggy",
            Condition::WornOut => "worn out",
            Condition::Diabetic => "diabetic",
            Condition::Dependent => "dependent",
            Condition::InRehab => "in rehab",
            Condition::WellRested => "well rested",
        }
    }
}

/// One bit per [`Condition`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub struct Conditions(pub u16);

/// Most conditions named on one line before the rest is counted.
const CONDITIONS_NAMED: usize = 2;

impl Conditions {
    pub fn has(self, c: Condition) -> bool {
        self.0 & (1 << c as u8) != 0
    }

    pub fn iter(self) -> impl Iterator<Item = Condition> {
        Condition::ALL.into_iter().filter(move |&c| self.has(c))
    }

    /// The most pressing conditions that fit `cols`, then " +n" for the rest.
    pub fn line(self, cols: usize) -> String<24> {
        let n = self.iter().count();
        for k in (1..=n.min(CONDITIONS_NAMED)).rev() {
            let mut out: String<24> = String::new();
            for (i, c) in self.iter().take(k).enumerate() {
                if i > 0 {
                    let _ = out.push_str(", ");
                }
                let _ = out.push_str(c.text());
            }
            if n > k {
                let mut rest: String<8> = String::new();
                let _ = core::fmt::write(&mut rest, format_args!(" +{}", n - k));
                let _ = out.push_str(&rest);
            }
            if out.len() <= cols {
                return out;
            }
        }
        String::new()
    }
}

/// The lasting conditions of the pet right now.
pub fn conditions(s: &GameState) -> Conditions {
    let injured = s.injured_min > 0;
    let bits = [
        (Condition::Injured, injured),
        (
            Condition::Sore,
            s.strain as i32 > SORE_PCT as i32 * P && !injured,
        ),
        (Condition::HungOver, s.hangover_min > 0),
        (Condition::Groggy, s.groggy_min > 0),
        (Condition::WornOut, s.deprived),
        (Condition::Diabetic, s.diabetic),
        (Condition::Dependent, s.dependent && s.rehab_min == 0),
        (Condition::InRehab, s.rehab_min > 0),
        (Condition::WellRested, s.rested_min > 0),
    ];
    Conditions(
        bits.into_iter()
            .fold(0, |b, (c, on)| b | (on as u16) << c as u8),
    )
}
