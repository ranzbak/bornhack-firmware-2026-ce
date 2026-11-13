//! Cause events: why a dose or a wake went badly.
//!
//! Rules push a [`Cause`] where they fire (`doses::apply`, `advance::wake`).
//! The queue is RAM-only and never saved; the UI drains it with
//! `GameState::take_causes` and shows the words as a toast and a diary line.

use heapless::Deque;

/// What a strained or torn muscle blames.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StrainWhy {
    Hungry,
    Thirsty,
    Tired,
    Heavy,
    WornOut,
    Sore,
}

impl StrainWhy {
    pub const ALL: [StrainWhy; 6] = [
        StrainWhy::Hungry,
        StrainWhy::Thirsty,
        StrainWhy::Tired,
        StrainWhy::Heavy,
        StrainWhy::WornOut,
        StrainWhy::Sore,
    ];

    fn why(self) -> &'static str {
        match self {
            StrainWhy::Hungry => "sport: hungry",
            StrainWhy::Thirsty => "sport: thirsty",
            StrainWhy::Tired => "sport: tired",
            StrainWhy::Heavy => "sport: heavy",
            StrainWhy::WornOut => "sport: worn out",
            StrainWhy::Sore => "sport: sore",
        }
    }
}

/// What kept a sleeper from resting (see `regime::unsettled_by`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unsettled {
    Drinks,
    Hungry,
    Stuffed,
    Thirsty,
    Waterlogged,
    Wired,
    Withdrawal,
    Unknown,
}

impl Unsettled {
    pub const ALL: [Unsettled; 8] = [
        Unsettled::Drinks,
        Unsettled::Hungry,
        Unsettled::Stuffed,
        Unsettled::Thirsty,
        Unsettled::Waterlogged,
        Unsettled::Wired,
        Unsettled::Withdrawal,
        Unsettled::Unknown,
    ];

    fn why(self) -> &'static str {
        match self {
            Unsettled::Drinks => "late drinks",
            Unsettled::Hungry => "hungry",
            Unsettled::Stuffed => "stuffed",
            Unsettled::Thirsty => "thirsty",
            Unsettled::Waterlogged => "waterlogged",
            Unsettled::Wired => "wired",
            Unsettled::Withdrawal => "withdrawal",
            Unsettled::Unknown => "slept badly",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    FoodComa,
    TreatTooSoon,
    NoTreatBuzz,
    DrinkDulled,
    OneTooMany,
    DryMouth,
    Relapse,
    Sloshing,
    TornMuscle(StrainWhy),
    Strained(StrainWhy),
    Grumpy,
    BadNight(Unsettled),
    Groggy,
    WornOut,
    StillHungOver,
    OnlyNumbed,
    OverRelaxed,
    WellRested,
}

/// The causes without a payload.
const PLAIN: [Cause; 15] = [
    Cause::FoodComa,
    Cause::TreatTooSoon,
    Cause::NoTreatBuzz,
    Cause::DrinkDulled,
    Cause::OneTooMany,
    Cause::DryMouth,
    Cause::Relapse,
    Cause::Sloshing,
    Cause::Grumpy,
    Cause::Groggy,
    Cause::WornOut,
    Cause::StillHungOver,
    Cause::OnlyNumbed,
    Cause::OverRelaxed,
    Cause::WellRested,
];

impl Cause {
    /// The short headline of a toast (at most 12 columns).
    pub fn what(self) -> &'static str {
        match self {
            Cause::FoodComa => "food coma",
            Cause::TreatTooSoon | Cause::DrinkDulled => "barely felt",
            Cause::NoTreatBuzz => "no buzz",
            Cause::OneTooMany => "sloppy",
            Cause::DryMouth => "dry mouth",
            Cause::Relapse => "relapse!",
            Cause::Sloshing => "sloshing",
            Cause::TornMuscle(_) => "torn muscle",
            Cause::Strained(_) => "strained",
            Cause::Grumpy => "grumpy",
            Cause::BadNight(_) => "bad night",
            Cause::Groggy => "groggy",
            Cause::WornOut => "worn out",
            Cause::StillHungOver => "hung over",
            Cause::OnlyNumbed => "only numbed",
            Cause::OverRelaxed => "bored",
            Cause::WellRested => "well rested",
        }
    }

    /// The reason line of a toast (at most 15 columns).
    pub fn why(self) -> &'static str {
        match self {
            Cause::FoodComa => "ate too much",
            Cause::TreatTooSoon => "treat too soon",
            Cause::NoTreatBuzz => "on Ozempic",
            Cause::DrinkDulled => "drunk already",
            Cause::OneTooMany => "one too many",
            Cause::DryMouth => "booze, no water",
            Cause::Relapse => "rehab lost",
            Cause::Sloshing => "drank too much",
            Cause::TornMuscle(w) | Cause::Strained(w) => w.why(),
            Cause::Grumpy => "woken too early",
            Cause::BadNight(u) => u.why(),
            Cause::Groggy => "overslept",
            Cause::WornOut => "short nights",
            Cause::StillHungOver => "needs water",
            Cause::OnlyNumbed => "not sick",
            Cause::OverRelaxed => "too much relax",
            Cause::WellRested => "good sleep",
        }
    }

    /// Everything but a good night is a warning.
    pub fn is_warning(self) -> bool {
        self != Cause::WellRested
    }

    /// Every variant, payloads included, once.
    pub fn all() -> impl Iterator<Item = Cause> {
        PLAIN
            .into_iter()
            .chain(StrainWhy::ALL.into_iter().map(Cause::TornMuscle))
            .chain(StrainWhy::ALL.into_iter().map(Cause::Strained))
            .chain(Unsettled::ALL.into_iter().map(Cause::BadNight))
    }

    /// A one-byte code, so a cause can cross a task boundary in an atomic.
    pub fn pack(self) -> u8 {
        // `all` is the one list; a code is the position in it.
        Cause::all().position(|c| c == self).unwrap_or(0) as u8
    }

    /// Inverse of [`pack`](Self::pack).
    pub fn unpack(v: u8) -> Option<Cause> {
        Cause::all().nth(v as usize)
    }
}

/// How many causes wait for the UI.
pub const CAUSE_QUEUE: usize = 4;

/// The newest [`CAUSE_QUEUE`] causes, oldest first. Never saved.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CauseQueue(Deque<Cause, CAUSE_QUEUE>);

impl CauseQueue {
    /// Queue a cause; when full the oldest is dropped.
    pub fn push(&mut self, c: Cause) {
        if self.0.is_full() {
            self.0.pop_front();
        }
        let _ = self.0.push_back(c);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Cause> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
