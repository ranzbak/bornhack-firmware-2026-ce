//! The signed NFC station text, `fix s=<id> <items>` (spec 19.2).
//!
//! Pure parsing, no alloc and no float. Applying a fix is a separate step.

use heapless::Vec;

use super::GameState;
use super::needs::{self, MAX, P};
use super::regime;
use super::thresholds as th;

/// Longest station text in bytes (spec 19.2).
pub const FIX_MAX_LEN: usize = 96;
/// Longest station id in bytes.
pub const STATION_ID_MAX: usize = 8;
/// 96 bytes hold at most 14 level items.
pub const MAX_LEVELS: usize = 16;

/// What a level item changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Full,
    Water,
    Energy,
    Fun,
    Dirt,
    Booze,
    Strain,
    Fit,
    Weight,
    Health,
}

impl Target {
    pub const ALL: [Target; 10] = [
        Target::Full,
        Target::Water,
        Target::Energy,
        Target::Fun,
        Target::Dirt,
        Target::Booze,
        Target::Strain,
        Target::Fit,
        Target::Weight,
        Target::Health,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Target::Full => "full",
            Target::Water => "water",
            Target::Energy => "energy",
            Target::Fun => "fun",
            Target::Dirt => "dirt",
            Target::Booze => "booze",
            Target::Strain => "strain",
            Target::Fit => "fit",
            Target::Weight => "weight",
            Target::Health => "health",
        }
    }
}

/// How a level item moves its target.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Set,
    AtLeast,
    AtMost,
    Add,
    Sub,
}

impl Op {
    pub fn ch(self) -> char {
        match self {
            Op::Set => '=',
            Op::AtLeast => '>',
            Op::AtMost => '<',
            Op::Add => '+',
            Op::Sub => '-',
        }
    }

    fn from_byte(b: u8) -> Option<Op> {
        match b {
            b'=' => Some(Op::Set),
            b'>' => Some(Op::AtLeast),
            b'<' => Some(Op::AtMost),
            b'+' => Some(Op::Add),
            b'-' => Some(Op::Sub),
            _ => None,
        }
    }
}

/// One level item, `pct` in 0..=100.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Level {
    pub target: Target,
    pub op: Op,
    pub pct: u8,
}

/// Bits of `Fix::clear`, in canonical order.
pub mod cond {
    pub const HANGOVER: u8 = 1;
    pub const INJURY: u8 = 2;
    pub const GROGGY: u8 = 4;
    pub const DEPRIVED: u8 = 8;
    pub const DEPENDENCE: u8 = 16;
    pub const DIABETES: u8 = 32;
    pub const CHRONIC: u8 = DEPENDENCE | DIABETES;
    pub const NAMES: [(&str, u8); 6] = [
        ("hangover", HANGOVER),
        ("injury", INJURY),
        ("groggy", GROGGY),
        ("deprived", DEPRIVED),
        ("dependence", DEPENDENCE),
        ("diabetes", DIABETES),
    ];
}

/// A parsed station message.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fix {
    id: [u8; STATION_ID_MAX],
    id_len: u8,
    pub levels: Vec<Level, MAX_LEVELS>,
    pub clear: u8,
    pub cd_min: Option<u16>,
}

impl Fix {
    /// `id` must be 1..=8 bytes of [a-z0-9] (callers: the parser, tests). Panics otherwise.
    pub fn new(id: &[u8]) -> Fix {
        assert!(valid_id(id), "station id must be 1..=8 bytes of [a-z0-9]");
        let mut buf = [0u8; STATION_ID_MAX];
        buf[..id.len()].copy_from_slice(id);
        Fix {
            id: buf,
            id_len: id.len() as u8,
            levels: Vec::new(),
            clear: 0,
            cd_min: None,
        }
    }

    pub fn id(&self) -> &[u8] {
        &self.id[..self.id_len as usize]
    }

    /// Something the badge acts on: a level item or a known condition (`cd=` alone is not).
    pub fn understood(&self) -> bool {
        !self.levels.is_empty() || self.clear != 0
    }

    /// The normal form the shared vectors and `scripts/station.py` print.
    #[cfg(any(test, feature = "simulator"))]
    pub fn canonical(&self) -> heapless::String<FIX_MAX_LEN> {
        use core::fmt::Write;
        // A canonical form is never longer than the 96-byte text it came from,
        // so these writes cannot overflow.
        let mut s = heapless::String::new();
        let _ = write!(s, "fix s={}", core::str::from_utf8(self.id()).unwrap_or(""));
        for l in &self.levels {
            let _ = write!(s, " {}{}{}", l.target.name(), l.op.ch(), l.pct);
        }
        if self.clear != 0 {
            let _ = s.push_str(" clear=");
            let mut first = true;
            for (name, bit) in cond::NAMES {
                if self.clear & bit != 0 {
                    if !first {
                        let _ = s.push(',');
                    }
                    first = false;
                    let _ = s.push_str(name);
                }
            }
        }
        if let Some(n) = self.cd_min {
            let _ = write!(s, " cd={n}");
        }
        s
    }
}

/// Why a text that starts like a station message is refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reject {
    Unreadable,
    BadId,
    BadItem,
    Over100,
    Duplicate,
    NothingKnown,
}

impl Reject {
    pub fn code(self) -> &'static str {
        match self {
            Reject::Unreadable => "unreadable",
            Reject::BadId => "bad_id",
            Reject::BadItem => "bad_item",
            Reject::Over100 => "over_100",
            Reject::Duplicate => "duplicate",
            Reject::NothingKnown => "nothing_known",
        }
    }
}

fn valid_id(id: &[u8]) -> bool {
    (1..=STATION_ID_MAX).contains(&id.len())
        && id.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Trim ASCII whitespace, lowercase, turn every run of ASCII whitespace into one space.
/// None when the result is empty, holds a non-ASCII byte, or is longer than FIX_MAX_LEN.
pub fn normalize(text: &[u8]) -> Option<Vec<u8, FIX_MAX_LEN>> {
    let mut out: Vec<u8, FIX_MAX_LEN> = Vec::new();
    let mut gap = false;
    for &b in text {
        if !b.is_ascii() {
            return None;
        }
        if b.is_ascii_whitespace() {
            gap = true;
            continue;
        }
        if gap && !out.is_empty() {
            out.push(b' ').ok()?;
        }
        gap = false;
        out.push(b.to_ascii_lowercase()).ok()?;
    }
    if out.is_empty() { None } else { Some(out) }
}

/// The old phrases, as station messages.
pub const ALIASES: [(&str, &str); 4] = [
    ("more food", "fix s=food full>60 water>60"),
    (
        "more drugs",
        "fix s=drugs health>100 booze<20 clear=hangover,injury,groggy",
    ),
    ("sleep like a bear", "fix s=rest energy>100 clear=groggy,deprived"),
    ("more inspiration", "fix s=inspire fun>60"),
];

/// The alias expansion of a normalised text, or the text itself.
pub fn expand_alias(normalized: &[u8]) -> &[u8] {
    for (phrase, fix) in ALIASES {
        if phrase.as_bytes() == normalized {
            return fix.as_bytes();
        }
    }
    normalized
}

fn target_from(name: &[u8]) -> Option<Target> {
    Target::ALL.into_iter().find(|t| t.name().as_bytes() == name)
}

/// 1..=max_digits ASCII digits (leading zeros allowed) as a number.
fn digits(s: &[u8], max_digits: usize) -> Option<u16> {
    if s.is_empty() || s.len() > max_digits || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(s.iter().fold(0u16, |n, d| n * 10 + u16::from(d - b'0')))
}

/// Ok(None): not a station message (the first word is not "fix").
pub fn parse(normalized: &[u8]) -> Result<Option<Fix>, Reject> {
    let mut tokens = normalized.split(|&b| b == b' ');
    if tokens.next() != Some(b"fix") {
        return Ok(None);
    }
    let id = tokens
        .next()
        .and_then(|t| t.strip_prefix(b"s="))
        .filter(|id| valid_id(id))
        .ok_or(Reject::BadId)?;
    let mut fix = Fix::new(id);
    let mut cd_seen = false;
    for tok in tokens {
        let at = tok
            .iter()
            .position(|&b| Op::from_byte(b).is_some())
            .ok_or(Reject::BadItem)?;
        let (name, rest) = (&tok[..at], &tok[at + 1..]);
        let op = Op::from_byte(tok[at]).ok_or(Reject::BadItem)?;
        if name.is_empty() || rest.is_empty() || !name.iter().all(u8::is_ascii_lowercase) {
            return Err(Reject::BadItem);
        }
        if let Some(target) = target_from(name) {
            let v = digits(rest, 3).ok_or(Reject::BadItem)?;
            if v > 100 {
                return Err(Reject::Over100);
            }
            let level = Level { target, op, pct: v as u8 };
            fix.levels.push(level).map_err(|_| Reject::BadItem)?;
        } else if name == b"clear" {
            if op != Op::Set {
                return Err(Reject::BadItem);
            }
            for word in rest.split(|&b| b == b',') {
                if word.is_empty() || !word.iter().all(u8::is_ascii_lowercase) {
                    return Err(Reject::BadItem);
                }
                for (known, bit) in cond::NAMES {
                    if known.as_bytes() == word {
                        fix.clear |= bit;
                    }
                }
            }
        } else if name == b"cd" {
            if op != Op::Set {
                return Err(Reject::BadItem);
            }
            let n = digits(rest, 4).ok_or(Reject::BadItem)?;
            if cd_seen {
                return Err(Reject::Duplicate);
            }
            cd_seen = true;
            fix.cd_min = Some(n);
        } else if name == b"s" {
            // The id was the second token; a second `s` is a duplicate.
            return Err(Reject::Duplicate);
        }
    }
    if fix.understood() {
        Ok(Some(fix))
    } else {
        Err(Reject::NothingKnown)
    }
}

/// normalize, expand_alias, parse. A text normalize refuses is Err(Unreadable) when
/// its trimmed, lowercased start is "fix" followed by a space or the end, else Ok(None).
pub fn parse_text(text: &[u8]) -> Result<Option<Fix>, Reject> {
    match normalize(text) {
        Some(n) => parse(expand_alias(&n)),
        None => {
            let start = text.trim_ascii_start();
            let is_fix = start.len() >= 3
                && start[..3].eq_ignore_ascii_case(b"fix")
                && start.get(3).is_none_or(|b| b.is_ascii_whitespace());
            if is_fix { Err(Reject::Unreadable) } else { Ok(None) }
        }
    }
}

/// A percent as a level; 100 is `MAX`.
fn lvl(pct: u8) -> i32 {
    if pct >= 100 { MAX } else { pct as i32 * P }
}

/// The levels a station may move `t` between, `[lo, hi]` (level units). The edges are the
/// pet's health-damage lines, so landing on an edge is safe (`regime::health_damage` uses
/// strict comparisons).
pub fn safe_range(s: &GameState, t: Target) -> (i32, i32) {
    match t {
        Target::Full | Target::Water => (lvl(th::DMG_NEED_LOW_PCT()), lvl(th::DMG_NEED_HIGH_PCT())),
        Target::Energy => (lvl(th::DMG_ENERGY_PCT()), MAX),
        Target::Fun => (0, MAX),
        Target::Dirt => (0, lvl(th::DMG_DIRT_PCT())),
        Target::Booze => (0, needs::booze_danger()),
        Target::Strain => (0, needs::strain_inj()),
        Target::Fit => (0, lvl(th::STATION_FIT_CAP_PCT())),
        Target::Weight => (lvl(th::DMG_WEIGHT_LOW_PCT()), lvl(th::DMG_WEIGHT_HIGH_PCT())),
        Target::Health => (lvl(needs::HEALTH_DANGER_PCT), regime::health_ceiling(s)),
    }
}

fn level_mut(s: &mut GameState, t: Target) -> &mut u16 {
    match t {
        Target::Full => &mut s.fullness,
        Target::Water => &mut s.hydration,
        Target::Energy => &mut s.energy,
        Target::Fun => &mut s.fun,
        Target::Dirt => &mut s.dirt,
        Target::Booze => &mut s.booze,
        Target::Strain => &mut s.strain,
        Target::Fit => &mut s.fitness,
        Target::Weight => &mut s.weight,
        Target::Health => &mut s.health,
    }
}

/// Apply now: conditions first, then levels in message order, then the mood bump. No
/// catch-up, no wake, no causes.
pub fn apply(s: &mut GameState, fix: &Fix) {
    if fix.clear & cond::HANGOVER != 0 {
        s.hangover_min = 0;
    }
    if fix.clear & cond::INJURY != 0 {
        s.injured_min = 0;
    }
    if fix.clear & cond::GROGGY != 0 {
        s.groggy_min = 0;
    }
    if fix.clear & cond::DEPRIVED != 0 {
        s.deprived = false;
    }
    if fix.clear & cond::DEPENDENCE != 0 {
        s.dependent = false;
        s.dependence_min = 0;
        s.rehab_min = 0;
        s.recovering_min = th::RECOVERING_MIN();
    }
    if fix.clear & cond::DIABETES != 0 {
        s.diabetic = false;
        s.diabetes_min = 0;
        s.reversal_min = 0;
    }
    for l in &fix.levels {
        let (lo, hi) = safe_range(s, l.target);
        let v = lvl(l.pct);
        let cell = level_mut(s, l.target);
        let cur = *cell as i32;
        let want = match l.op {
            Op::Set => v,
            Op::AtLeast => cur.max(v),
            Op::AtMost => cur.min(v),
            Op::Add => cur + v,
            Op::Sub => cur - v,
        };
        // The clamp limits how far the item moves the need, never the other way.
        let new = if want > cur { want.min(hi.max(cur)) } else { want.max(lo.min(cur)) };
        *cell = new as u16;
        if l.target == Target::Energy && matches!(l.op, Op::Set | Op::AtLeast | Op::Add) {
            s.no_groggy = true;
        }
    }
    s.mood = (s.mood as i32 + lvl(th::STATION_MOOD_PCT())).min(MAX) as u16;
}
