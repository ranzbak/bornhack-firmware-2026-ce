//! BornPets in-game modal overlay.
//!
//! A modal is a pop-over window drawn on top of the game screen when the player
//! activates an icon.  It shows a short action list; the selected item is
//! inverted.  The cancel button always dismisses it.
//!
//! ```text
//! ┌──────────────────────────────┐  y = 10
//! │▓▓▓▓▓▓▓▓▓ Feed ▓▓▓▓▓▓▓▓▓▓▓▓▓│  title bar (black fill, white text)
//! ├──────────────────────────────┤  y = 30
//! │  Feed now                    │
//! │ ►► Cancel ◄◄                 │  ← selected item, inverted
//! └──────────────────────────────┘  y = 141
//! ```
//!
//! 10 px margin on all sides keeps the underlying game screen visible.

use core::sync::atomic::{AtomicU8, Ordering};

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, Triangle};
use embedded_graphics::text::{Text};

use super::engine::DebugPoke;
use super::nav::Row;
use crate::ui::{self, TEXT_BOLD_BLACK, TEXT_BOLD_WHITE};
use crate::{BLACK, TriColor};

// ── Modal kind
// ────────────────────────────────────────────────────────────────

/// Which in-game modal is currently open.  Stored as a `u8` in `MODAL_KIND`.
///
/// Layout:
///   Top row (info/meta):    Stats, Hibernate, Exercise, (empty)
///   Bottom row (actions):   Feed,  Heal,      Play,     Rest
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModalKind {
    None = 0,
    // Top row — info / meta.
    Stats = 1,     // top row, col 0
    Hibernate = 2, // top row, col 1
    // Bottom row — actions.
    Feed = 3, // bot row, col 0
    Heal = 4, // bot row, col 1
    Play = 5, // bot row, col 2
    Rest = 6, // bot row, col 3
    // Sub-modal opened from Play.
    Music = 7,
    Exercise = 8, // top row, col 2
    // Hidden — opened only via the button sequence in `debug_cheats`,
    // not reachable from any icon.
    Debug = 9,
    Drink = 10, // top row, col 3
}

impl ModalKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Stats,
            2 => Self::Hibernate,
            3 => Self::Feed,
            4 => Self::Heal,
            5 => Self::Play,
            6 => Self::Rest,
            7 => Self::Music,
            8 => Self::Exercise,
            9 => Self::Debug,
            10 => Self::Drink,
            _ => Self::None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Stats => "Stats",
            Self::Hibernate => "Hibernate",
            Self::Feed => "Feed",
            Self::Heal => "Heal",
            Self::Play => "Play",
            Self::Rest => "Rest",
            Self::Music => "Music",
            Self::Exercise => "Exercise",
            Self::Debug => "Debug",
            Self::Drink => "Drink",
        }
    }

    fn items(self) -> &'static [Item] {
        match self {
            Self::Stats => &[
                Item::ViewStats,
                Item::RolledStats,
                Item::HealthStatus,
                Item::Friends,
                Item::Cancel,
            ],
            Self::Hibernate => &[Item::Hibernate, Item::WakeUp, Item::Cancel],
            Self::Feed => &[
                Item::FeedFood(super::engine::FoodKind::Salad),
                Item::FeedFood(super::engine::FoodKind::Apple),
                Item::FeedFood(super::engine::FoodKind::Frikandel),
                Item::FeedFood(super::engine::FoodKind::Pizza),
                Item::FeedFood(super::engine::FoodKind::Cake),
                Item::Cancel,
            ],
            Self::Heal => &[
                Item::GiveMedicine,
                Item::GiveMedication,
                Item::Ozempic,
                Item::Rehab,
                Item::Cancel,
            ],
            Self::Exercise => &[Item::ExerciseNow, Item::Cancel],
            Self::Debug => &DEBUG_ITEMS,
            Self::Drink => &[
                Item::DrinkChoice(super::engine::DrinkKind::Water),
                Item::DrinkChoice(super::engine::DrinkKind::Cola),
                Item::DrinkChoice(super::engine::DrinkKind::Beer),
                Item::DrinkChoice(super::engine::DrinkKind::Wine),
                Item::DrinkChoice(super::engine::DrinkKind::Whiskey),
                Item::Cancel,
            ],
            Self::Play => {
                if super::lifecycle::money_enabled() {
                    &[
                        Item::Battle,
                        Item::PlayNow,
                        Item::OnlyPets,
                        Item::PlayMusic,
                        Item::TicTacToe,
                        Item::LightsOut,
                        Item::BlackHole,
                        Item::Nim,
                        Item::BornJeweled,
                        Item::Cancel,
                    ]
                } else {
                    &[
                        Item::Battle,
                        Item::PlayNow,
                        Item::PlayMusic,
                        Item::TicTacToe,
                        Item::LightsOut,
                        Item::BlackHole,
                        Item::Nim,
                        Item::BornJeweled,
                        Item::Cancel,
                    ]
                }
            }
            Self::Music => &[
                Item::Song(crate::SONG_STARTUP_INDEX),
                Item::Song(crate::SONG_RICKROLL_INDEX),
                Item::Song(crate::SONG_IMPERIAL_MARCH_INDEX),
                Item::Song(crate::SONG_SANDSTORM_INDEX),
                Item::Song(crate::SONG_PINK_PANTHER_INDEX),
                Item::Song(crate::SONG_TROLOLO_INDEX),
                Item::Song(crate::SONG_DAISY_BELL_INDEX),
                Item::Song(crate::SONG_NOKIA_INDEX),
                Item::Song(crate::SONG_OVER_THE_HORIZON_INDEX),
                Item::Cancel,
            ],
            Self::Rest => &[
                Item::Sleep,
                Item::Relax,
                Item::Wash,
                Item::WakeFromSleep,
                Item::Cancel,
            ],
            Self::None => &[],
        }
    }
}

// ── Items
// ──────────────────────────────────────────────────────────────────────

/// One row in the modal's item list.  Each variant carries its own
/// label, availability check, cooldown source, and activation handler
/// — no string matching anywhere else in the file.
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Cancel,
    ViewStats,
    RolledStats,
    HealthStatus,
    Friends,
    Battle,
    FeedFood(super::engine::FoodKind),
    GiveMedicine,
    GiveMedication,
    Ozempic,
    ExerciseNow,
    /// A row of [`DEBUG_ROWS`] (the hidden Konami-code page).
    Debug(u8),
    DrinkChoice(super::engine::DrinkKind),
    Rehab,
    Sleep,
    Relax,
    Wash,
    /// End a sleep (distinct from `WakeUp`, the hibernation defrost).
    WakeFromSleep,
    PlayNow,
    OnlyPets,
    TicTacToe,
    LightsOut,
    BlackHole,
    Nim,
    BornJeweled,
    PlayMusic,
    /// Buzzer song index passed to [`crate::fw::buzzer::play`].
    Song(u8),
    Hibernate,
    WakeUp,
}

/// What a debug row does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DebugRow {
    /// Show `engine::DebugInfo` as a toast.
    Info,
    /// Fast-forward this many minutes.
    Skip(u32),
    Poke(DebugPoke),
    /// Zero the Battle record, this pet's and every friend's.
    ResetBattle,
}

/// The hidden debug page: label and action of every row, in menu order. The
/// page list and the dispatch both read this one table.
const DEBUG_ROWS: [(&str, DebugRow); 23] = [
    ("Engine info", DebugRow::Info),
    ("Skip 10 min", DebugRow::Skip(10)),
    ("Skip 1 hour", DebugRow::Skip(60)),
    ("Skip 1 day", DebugRow::Skip(24 * 60)),
    ("Comfort", DebugRow::Poke(DebugPoke::Comfort)),
    ("Hungry", DebugRow::Poke(DebugPoke::Hungry)),
    ("Thirsty", DebugRow::Poke(DebugPoke::Thirsty)),
    ("Stuffed", DebugRow::Poke(DebugPoke::Stuffed)),
    ("Tired", DebugRow::Poke(DebugPoke::Tired)),
    ("Bored", DebugRow::Poke(DebugPoke::Bored)),
    ("Dirty", DebugRow::Poke(DebugPoke::Dirty)),
    ("Fat", DebugRow::Poke(DebugPoke::Fat)),
    ("Thin", DebugRow::Poke(DebugPoke::Thin)),
    ("Unfit", DebugRow::Poke(DebugPoke::Unfit)),
    ("Drunk", DebugRow::Poke(DebugPoke::Drunk)),
    ("Hung over", DebugRow::Poke(DebugPoke::HungOver)),
    ("Groggy", DebugRow::Poke(DebugPoke::Groggy)),
    ("Injured", DebugRow::Poke(DebugPoke::Injured)),
    ("Diabetic", DebugRow::Poke(DebugPoke::Diabetic)),
    ("Dependent", DebugRow::Poke(DebugPoke::Dependent)),
    ("Clear conditions", DebugRow::Poke(DebugPoke::ClearConditions)),
    ("Clear cooldowns", DebugRow::Poke(DebugPoke::ClearCooldowns)),
    ("Reset battle record", DebugRow::ResetBattle),
];

/// The debug page's items: one per table row, then Cancel.
static DEBUG_ITEMS: [Item; DEBUG_ROWS.len() + 1] = {
    let mut items = [Item::Cancel; DEBUG_ROWS.len() + 1];
    let mut i = 0;
    while i < DEBUG_ROWS.len() {
        items[i] = Item::Debug(i as u8);
        i += 1;
    }
    items
};

/// Do one debug row. A poke the pet cannot take shows its reason (the one
/// every refused action shows) and leaves the menu open.
fn run_debug_row(row: DebugRow) {
    use super::lifecycle;
    match row {
        DebugRow::Info => super::show_toast(super::Toast::DebugInfo),
        DebugRow::Skip(minutes) => {
            lifecycle::debug_skip_minutes(minutes);
            super::show_toast(super::Toast::DebugCheat);
        }
        DebugRow::Poke(poke) => match lifecycle::debug_poke(poke) {
            Ok(()) => super::show_toast(super::Toast::DebugCheat),
            Err(why) => return super::show_reason(why),
        },
        DebugRow::ResetBattle => {
            lifecycle::debug_reset_battle_record();
            super::show_toast(super::Toast::DebugCheat);
        }
    }
    close();
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Self::Cancel => "Cancel",
            Self::ViewStats => "View stats",
            Self::RolledStats => "Rolled stats",
            Self::HealthStatus => "Health status",
            Self::Friends => "Friends",
            Self::Battle => "Battle",
            Self::FeedFood(food) => food.label(),
            Self::GiveMedicine => "Aspirine",
            Self::GiveMedication => "Insulin",
            Self::Ozempic => "Ozempic",
            Self::ExerciseNow => "Exercise now",
            Self::Debug(row) => DEBUG_ROWS[row as usize].0,
            Self::DrinkChoice(drink) => drink.label(),
            Self::Rehab => "Rehab",
            Self::Sleep => "Sleep",
            Self::Relax => "Relax",
            Self::Wash => "Wash",
            Self::WakeFromSleep => "Wake up",
            Self::PlayNow => "Play now",
            Self::OnlyPets => "Only pets",
            Self::TicTacToe => "Tic Tac Toe",
            Self::LightsOut => "Lights Out",
            Self::BlackHole => "Black Hole",
            Self::Nim => "Nim",
            Self::BornJeweled => "BornJeweled",
            Self::PlayMusic => "Play music",
            Self::Song(crate::SONG_STARTUP_INDEX) => "Startup",
            Self::Song(crate::SONG_RICKROLL_INDEX) => "Rickroll",
            Self::Song(crate::SONG_IMPERIAL_MARCH_INDEX) => "Imp. March",
            Self::Song(crate::SONG_SANDSTORM_INDEX) => "Sandstorm",
            Self::Song(crate::SONG_PINK_PANTHER_INDEX) => "Pink Panther",
            Self::Song(crate::SONG_TROLOLO_INDEX) => "Trololo",
            Self::Song(crate::SONG_DAISY_BELL_INDEX) => "Daisy Bell",
            Self::Song(crate::SONG_NOKIA_INDEX) => "Nokia Tune",
            Self::Song(crate::SONG_OVER_THE_HORIZON_INDEX) => "Samsung",
            Self::Song(_) => "?",
            Self::Hibernate => "Hibernate",
            Self::WakeUp => "Defrost",
        }
    }

    /// HAX price of this item in the current mode, if it costs money.
    /// Shown as a " NNH" suffix in the menu (only when money mode is
    /// on). Only-pets earns HAX rather than costing it, so it has no
    /// price. Takes `stats` so Hard (US) mode can report the actual
    /// effective price rather than the normal-mode base.
    fn hax_price(self, stats: &super::engine::PetStats) -> Option<u32> {
        match self {
            Self::FeedFood(food) => Some(food.hax_price(stats.hard_mode)),
            Self::DrinkChoice(drink) => Some(drink.hax_price()),
            Self::GiveMedicine => Some(super::engine::ASPIRINE_HAX_COST),
            Self::PlayNow => Some(super::engine::PLAY_HAX_COST),
            Self::GiveMedication | Self::Ozempic => {
                Some(super::engine::medication_price(stats.hard_mode))
            }
            Self::Rehab => Some(super::engine::rehab_price(stats.hard_mode)),
            _ => None,
        }
    }

    /// Is the action currently available?  Cooldown-gated items
    /// return false until the cooldown decays to 0.
    ///
    /// While the pet is hibernating EVERY action is locked except the
    /// Defrost button, plus passive info screens (Cancel / ViewStats /
    /// RolledStats).  The pet must be defrosted before mini-games or
    /// stat actions can run again.
    fn available(self, stats: &super::engine::PetStats) -> bool {
        // Debug rows are always selectable: a poke the pet cannot take says
        // why when it runs, and the skips work on a hibernating pet too.
        if stats.hibernating && !matches!(self, Self::Debug(_)) {
            return matches!(
                self,
                Self::Cancel | Self::ViewStats | Self::RolledStats | Self::WakeUp,
            );
        }
        match self {
            Self::Cancel
            | Self::ViewStats
            | Self::RolledStats
            | Self::HealthStatus
            | Self::Friends
            | Self::PlayMusic
            | Self::Song(_)
            | Self::Debug(_) => true,
            Self::DrinkChoice(drink) => {
                stats.can_drink
                    && (drink != super::engine::DrinkKind::Water || stats.can_drink_water)
                    && (!stats.money_enabled || stats.money >= drink.hax_price())
            }
            Self::Rehab => {
                stats.can_rehab
                    && (!stats.money_enabled
                        || stats.money >= super::engine::rehab_price(stats.hard_mode))
            }
            Self::Battle => stats.can_battle,
            Self::FeedFood(food) => {
                stats.can_feed
                    && (!stats.money_enabled || stats.money >= food.hax_price(stats.hard_mode))
            }
            Self::GiveMedicine => {
                stats.can_heal
                    && (!stats.money_enabled || stats.money >= super::engine::ASPIRINE_HAX_COST)
            }
            Self::GiveMedication => {
                stats.can_medicate
                    && (!stats.money_enabled
                        || stats.money >= super::engine::medication_price(stats.hard_mode))
            }
            Self::Ozempic => {
                stats.can_ozempic
                    && (!stats.money_enabled
                        || stats.money >= super::engine::medication_price(stats.hard_mode))
            }
            Self::ExerciseNow => stats.can_exercise,
            Self::Sleep => stats.can_sleep,
            Self::Relax => stats.can_relax,
            Self::Wash => stats.can_wash,
            Self::WakeFromSleep => stats.can_wake,
            Self::PlayNow => {
                stats.can_play
                    && (!stats.money_enabled || stats.money >= super::engine::PLAY_HAX_COST)
            }
            // Never gated — the escape hatch when broke.
            Self::OnlyPets => stats.can_only_pets,
            Self::TicTacToe => stats.can_play_tictactoe,
            Self::LightsOut => stats.can_play_lightsout,
            Self::BlackHole => stats.can_play_blackhole,
            Self::Nim => stats.can_play_nim,
            Self::BornJeweled => stats.can_play_bornjeweled,
            Self::Hibernate => true,
            Self::WakeUp => false,
        }
    }

    /// Can the cursor rest on this row?  Cancel always can (so every
    /// list has at least one selectable row); everything else needs an
    /// engine snapshot that says it's available.  Missing snapshot
    /// (very early boot) locks out everything but Cancel.
    fn selectable(self, stats: Option<&super::engine::PetStats>) -> bool {
        matches!(self, Self::Cancel) || stats.is_some_and(|s| self.available(s))
    }

    /// Time-until-ready in 10-second ticks for a disabled item: the running
    /// action's remaining ticks for its own row, otherwise the wait the
    /// engine itself reports (`Reason::Wait`, the same one the toast shows).
    /// 0 = ready / not gated by a wait.
    fn cooldown_ticks(self, stats: &super::engine::PetStats) -> u32 {
        use super::engine::doses::Reason;
        if self.action().is_some_and(|(a, _)| stats.active_action == Some(a)) {
            return stats.action_ticks_remaining as u32;
        }
        match self.reason(stats) {
            Reason::Wait(secs) => secs / 10,
            _ => 0,
        }
    }

    /// The engine action (and chosen drink) behind this row, if it is one.
    fn action(
        self,
    ) -> Option<(super::engine::Action, Option<super::engine::DrinkKind>)> {
        use super::engine::Action;
        Some(match self {
            Self::FeedFood(_) => (Action::Feed, None),
            Self::GiveMedicine => (Action::Heal, None),
            Self::GiveMedication => (Action::Medicate, None),
            Self::Ozempic => (Action::Ozempic, None),
            Self::ExerciseNow => (Action::Exercise, None),
            Self::DrinkChoice(d) => (Action::Drink, Some(d)),
            Self::Rehab => (Action::Rehab, None),
            Self::Relax => (Action::Relax, None),
            Self::Wash => (Action::Wash, None),
            Self::PlayNow => (Action::Play, None),
            Self::OnlyPets => (Action::OnlyPets, None),
            _ => return None,
        })
    }

    /// Why this row is disabled (or its starter just refused), asked of the
    /// engine; "can't afford" is what is left when the engine has no
    /// objection.
    fn reason(self, stats: &super::engine::PetStats) -> super::engine::doses::Reason {
        use super::engine::MiniGame;
        use super::engine::doses::{Reason, Refusal};
        use super::lifecycle;
        if stats.hibernating {
            return Reason::Hibernating;
        }
        let engine_says = match self {
            Self::Battle => lifecycle::game_refusal(None),
            Self::TicTacToe => lifecycle::game_refusal(Some(MiniGame::TicTacToe)),
            Self::LightsOut => lifecycle::game_refusal(Some(MiniGame::LightsOut)),
            Self::BlackHole => lifecycle::game_refusal(Some(MiniGame::BlackHole)),
            Self::Nim => lifecycle::game_refusal(Some(MiniGame::Nim)),
            Self::BornJeweled => lifecycle::game_refusal(Some(MiniGame::BornJeweled)),
            Self::Sleep => lifecycle::sleep_refusal(),
            Self::WakeFromSleep => (!stats.is_sleeping).then_some(Reason::Refused(Refusal::NoNeed)),
            _ => match self.action() {
                Some((a, d)) => lifecycle::refusal_for(a, d),
                None => None,
            },
        };
        match (engine_says, self.action()) {
            (Some(why), _) => why,
            (None, Some(_)) => Reason::CantAfford,
            (None, None) => Reason::Refused(Refusal::NoNeed),
        }
    }

    /// A starter just returned: a toast for success (and close), or the
    /// reason it refused (menu stays open).
    fn started(self, ok: bool, toast: super::Toast) {
        if ok {
            super::show_toast(toast);
            close();
        } else if let Some(stats) = super::lifecycle::cycle() {
            super::show_reason(self.reason(&stats));
        }
    }

    fn activate(self) {
        use super::lifecycle;
        match self {
            Self::Cancel => close(),
            Self::ViewStats => {
                STATS_VIEW.store(true, Ordering::Relaxed);
                super::note_live_view_drawn();
            }
            Self::RolledStats => {
                super::traits_view::open();
                close();
            }
            Self::HealthStatus => {
                super::health_view::open();
                close();
            }
            Self::Friends => {
                super::friends_view::open();
                close();
            }
            Self::Battle => {
                super::battle_view::open();
                close();
            }
            Self::FeedFood(food) => {
                self.started(lifecycle::feed(food), super::Toast::Feed);
            }
            Self::GiveMedicine => {
                self.started(lifecycle::heal(), super::Toast::Heal);
            }
            Self::GiveMedication => {
                self.started(lifecycle::medicate(), super::Toast::Medicate);
            }
            Self::Ozempic => {
                self.started(lifecycle::ozempic(), super::Toast::Ozempic);
            }
            Self::ExerciseNow => {
                self.started(lifecycle::exercise(), super::Toast::Exercise);
            }
            Self::Debug(row) => run_debug_row(DEBUG_ROWS[row as usize].1),
            Self::DrinkChoice(drink) => {
                let toast = if drink.is_alcoholic() { super::Toast::Drink } else { super::Toast::Refreshed };
                self.started(lifecycle::drink(drink), toast);
            }
            Self::Rehab => {
                self.started(lifecycle::rehab(), super::Toast::Rehab);
            }
            Self::Sleep => {
                self.started(lifecycle::sleep(), super::Toast::Sleep);
            }
            Self::Relax => {
                self.started(lifecycle::relax(), super::Toast::Relax);
            }
            Self::Wash => {
                self.started(lifecycle::wash(), super::Toast::Wash);
            }
            Self::WakeFromSleep => {
                self.started(lifecycle::wake(), super::Toast::Wake);
            }
            Self::PlayNow => {
                self.started(lifecycle::play(), super::Toast::Play);
            }
            Self::OnlyPets => {
                self.started(lifecycle::only_pets(), super::Toast::OnlyPets);
            }
            Self::TicTacToe => {
                super::tictactoe::open();
                close();
            }
            Self::LightsOut => {
                super::lightsout::open();
                close();
            }
            Self::BlackHole => {
                super::blackhole::open();
                close();
            }
            Self::Nim => {
                super::nim::open();
                close();
            }
            Self::BornJeweled => {
                super::bornjeweled::open();
                close();
            }
            Self::PlayMusic => open(ModalKind::Music),
            Self::Song(idx) => play_song(idx as usize),
            Self::Hibernate => {
                lifecycle::hibernate();
                super::show_toast(super::Toast::Hibernate);
                close();
            }
            Self::WakeUp => {
                lifecycle::wake_from_hibernation();
                super::show_toast(super::Toast::Wake);
                close();
            }
        }
    }
}

/// Format a cooldown count (ticks of 10 s each) as a " (50s)", " (59m)" or
/// " (2h05)" suffix for a disabled menu row.  Returns `None` when the cooldown is 0.
fn cooldown_string(ticks: u32) -> Option<heapless::String<12>> {
    if ticks == 0 {
        return None;
    }
    let mut s = heapless::String::new();
    let _ = core::fmt::Write::write_fmt(
        &mut s,
        format_args!(" ({})", super::engine::doses::fmt_wait(ticks.saturating_mul(10))),
    );
    Some(s)
}

/// Map an icon (row, col) to the modal it should open.
///
/// While the pet is hibernating, every action icon (bottom row) is a
/// no-op — only Stats and Hibernate (which holds the Defrost button)
/// can be opened.  Defence-in-depth alongside [`Item::available`].
pub fn kind_for_icon(row: Row, col: u8) -> ModalKind {
    if super::lifecycle::is_hibernating() {
        return match (row, col) {
            (Row::Top, 0) => ModalKind::Stats,
            (Row::Top, 1) => ModalKind::Hibernate,
            _ => ModalKind::None,
        };
    }
    match (row, col) {
        // Top row: info / meta.
        (Row::Top, 0) => ModalKind::Stats,
        (Row::Top, 1) => ModalKind::Hibernate,
        (Row::Top, 2) => ModalKind::Exercise,
        (Row::Top, 3) => ModalKind::Drink,
        (Row::Top, _) => ModalKind::None,
        // Bottom row: actions.
        (Row::Bottom, 0) => ModalKind::Feed,
        (Row::Bottom, 1) => ModalKind::Heal,
        (Row::Bottom, 2) => ModalKind::Play,
        (Row::Bottom, 3) => ModalKind::Rest,
        _ => ModalKind::None,
    }
}

// ── Global state
// ──────────────────────────────────────────────────────────────

static MODAL_KIND: AtomicU8 = AtomicU8::new(0);
static MODAL_POS: AtomicU8 = AtomicU8::new(0);
/// When true, the Stats modal shows stat bars instead of the menu list.
static STATS_VIEW: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

pub fn open(kind: ModalKind) {
    // Start on the first row that can be used, so a disabled first row
    // (e.g. Battle on cooldown) doesn't need to be stepped past.
    let stats = super::lifecycle::cycle();
    let items = kind.items();
    let pos = first_selectable(items.len(), |i| items[i].selectable(stats.as_ref()));
    MODAL_POS.store(pos as u8, Ordering::Relaxed);
    MODAL_KIND.store(kind as u8, Ordering::Relaxed);
    // Menu open → redraw the whole frame next refresh (grey-out overlay
    // covers everything); avoids partial-delta artifacts at the overlay edge.
    crate::FULL_REFRESH_PENDING.store(true, Ordering::Relaxed);
}

pub fn close() {
    STATS_VIEW.store(false, Ordering::Relaxed);
    MODAL_KIND.store(ModalKind::None as u8, Ordering::Relaxed);
    MODAL_POS.store(0, Ordering::Relaxed);
    // Menu close → redraw the whole frame (remove the overlay cleanly).
    crate::FULL_REFRESH_PENDING.store(true, Ordering::Relaxed);
}

/// A modal sub-view whose numbers change by themselves (the stats bars).
/// Task 5's diary and Task 9's hint view join it.
pub fn live_modal_view_open() -> bool {
    STATS_VIEW.load(Ordering::Relaxed)
}

pub fn is_open() -> bool {
    MODAL_KIND.load(Ordering::Relaxed) != 0
}

// ── Cursor navigation
// ─────────────────────────────────────────────────────────

/// Index of the first row `selectable` accepts, or 0 when none does.
fn first_selectable(len: usize, selectable: impl Fn(usize) -> bool) -> usize {
    (0..len).find(|&i| selectable(i)).unwrap_or(0)
}

/// The row after `from` (or before it when `forward` is false), wrapping
/// around the ends of a list of `len` rows.
fn step_cursor(len: usize, from: usize, forward: bool) -> usize {
    if len == 0 {
        from
    } else if forward {
        (from + 1) % len
    } else {
        (from + len - 1) % len
    }
}

/// Move the cursor one row up or down. Disabled rows are NOT skipped: the
/// player can rest on one and press it to be told why it is disabled.
fn move_cursor(forward: bool) {
    // While the stats-bar sub-view is showing there's no item list on
    // screen to move a cursor through — ignore Up/Down so it doesn't
    // silently advance MODAL_POS underneath the bars (surfacing as a
    // seemingly random row when the view is later dismissed back to the list).
    if STATS_VIEW.load(Ordering::Relaxed) {
        return;
    }
    let kind = ModalKind::from_u8(MODAL_KIND.load(Ordering::Relaxed));
    let items = kind.items();
    if items.is_empty() {
        return;
    }
    let pos = (MODAL_POS.load(Ordering::Relaxed) as usize).min(items.len() - 1);
    let next = step_cursor(items.len(), pos, forward);
    MODAL_POS.store(next as u8, Ordering::Relaxed);
}

pub fn cursor_up() {
    move_cursor(false);
}

pub fn cursor_down() {
    move_cursor(true);
}

/// Activate the currently selected item.
///
/// "Cancel" closes the modal.  Action items dispatch to the game engine
/// via `lifecycle`.  "View stats" opens the stats bar display.
pub fn activate() {
    // If stats view is showing, any activation closes it.
    if STATS_VIEW.load(Ordering::Relaxed) {
        STATS_VIEW.store(false, Ordering::Relaxed);
        return;
    }

    let kind = ModalKind::from_u8(MODAL_KIND.load(Ordering::Relaxed));
    let pos = MODAL_POS.load(Ordering::Relaxed) as usize;
    let items = kind.items();
    let Some(&item) = items.get(pos) else {
        return;
    };

    // Cancel is always allowed.  Everything else gates on the item's
    // own availability check.
    if !matches!(item, Item::Cancel) {
        let stats = match super::lifecycle::cycle() {
            Some(s) => s,
            None => return,
        };
        if !item.available(&stats) {
            // Never a silent no-op: say why this row is disabled.
            super::show_reason(item.reason(&stats));
            return;
        }
    }

    item.activate();
}

fn play_song(_index: usize) {
    #[cfg(feature = "embassy-core")]
    crate::fw::buzzer::play(_index);
    close();
}

// ── Drawing
// ───────────────────────────────────────────────────────────────────

const MARGIN: i32 = 10;
const MODAL_W: u32 = 132; // 152 - 2 × MARGIN
const MODAL_H: u32 = 132;
const BORDER: u32 = 2;
const TITLE_H: i32 = 18;
const ITEM_H: i32 = 16;

/// Draw the modal overlay.  Call this after `draw_screen_game` so it renders
/// on top.  Does nothing when no modal is open.
pub fn draw_modal<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let kind = ModalKind::from_u8(MODAL_KIND.load(Ordering::Relaxed));
    if kind == ModalKind::None {
        return Ok(());
    }

    // Stats view: show stat bars instead of the menu.
    if STATS_VIEW.load(Ordering::Relaxed) {
        return draw_stats_view(display);
    }

    let pos = MODAL_POS.load(Ordering::Relaxed) as usize;
    let items = kind.items();

    // White popover frame with 2 px black border.
    ui::draw_popover_frame(
        display,
        Point::new(MARGIN, MARGIN),
        Size::new(MODAL_W, MODAL_H),
        BORDER,
    )?;

    // Title bar — black fill, bold white text.
    let inner_x = MARGIN + BORDER as i32;
    let inner_y = MARGIN + BORDER as i32;
    let inner_w = MODAL_W - BORDER * 2;
    ui::draw_title_bar(display, kind.title(), Point::new(inner_x, inner_y), inner_w)?;

    // Item list
    let left_style = crate::ui::MIDDLE_LEFT;

    let list_x = inner_x;
    let list_y = inner_y + TITLE_H;
    let list_bottom = MARGIN + MODAL_H as i32 - BORDER as i32;

    let stats = super::lifecycle::cycle();

    // ── Scroll window ─────────────────────────────────────────────────
    // When the items list is taller than the visible area, reserve a
    // strip above and below for up/down arrow indicators and slide a
    // window over `items` so the selected `pos` is always visible.
    // Stateless: scroll position is derived from `pos` each frame.
    const ARROW_H: i32 = 10;
    let visible_no_arrows = ((list_bottom - list_y) / ITEM_H).max(1) as usize;
    let needs_scroll = items.len() > visible_no_arrows;
    let arrow_pad = if needs_scroll { ARROW_H } else { 0 };
    let inner_top = list_y + arrow_pad;
    let inner_bottom = list_bottom - arrow_pad;
    let visible_rows = ((inner_bottom - inner_top) / ITEM_H).max(1) as usize;
    let max_top = items.len().saturating_sub(visible_rows);
    // Center the selected row in the window when possible; clamp at
    // the start and end of the items list.
    let scroll_top = pos.saturating_sub(visible_rows / 2).min(max_top);

    for (vis_i, item) in items
        .iter()
        .enumerate()
        .skip(scroll_top)
        .take(visible_rows)
        .map(|(i, it)| (i - scroll_top, it))
    {
        let i = scroll_top + vis_i;
        let row_top = inner_top + vis_i as i32 * ITEM_H;
        let row_mid = row_top + ITEM_H / 2;
        if row_top + ITEM_H > inner_bottom {
            break;
        }

        // The cursor normally skips disabled rows, but one can still be
        // under it when it became unavailable while the modal was open
        // (e.g. a cooldown started elsewhere) — the "selected but locked
        // out" rendering below covers that case.
        let available = item.selectable(stats.as_ref());

        // Build display text: append " (Ns)" with the remaining wait when
        // the engine reports one.
        // 40 covers the longest label ("Reset battle record"/"Trigger
        // alcoholism", 19 bytes) plus an optional price suffix (" 15H",
        // 4 bytes) plus the longest cooldown suffix (" (72h00)", 8
        // bytes) with room to spare — heapless::push_str
        // silently no-ops past capacity instead of truncating, so this
        // must comfortably exceed the worst case, not just the common one.
        let mut display_label: heapless::String<40> = heapless::String::new();
        // US "Hard mode" reskins the Frikandel speciaal as a Hamburger.
        let label = match item {
            Item::FeedFood(super::engine::FoodKind::Frikandel)
                if stats.as_ref().is_some_and(|s| s.hard_mode) =>
            {
                "Hamburger"
            }
            _ => item.label(),
        };
        let _ = display_label.push_str(label);
        // Price suffix (" NNH") for priced items, only in money mode.
        if let Some(s) = stats.as_ref()
            && let Some(price) = item.hax_price(s)
            && s.money_enabled
        {
            let _ =
                core::fmt::Write::write_fmt(&mut display_label, format_args!(" {price}H"));
        }
        if !available {
            let suffix = stats
                .as_ref()
                .and_then(|s| cooldown_string(item.cooldown_ticks(s)));
            // Rows with no wait to show get no suffix: selecting one tells
            // the player why (R24).
            if let Some(s) = suffix {
                let _ = display_label.push_str(s.as_str());
            }
        }

        if i == pos && available {
            // Selected and available: inverted row.
            Rectangle::new(
                Point::new(inner_x, row_top),
                Size::new(inner_w, ITEM_H as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(BLACK))
            .draw(display)?;
            Text::with_text_style(
                display_label.as_str(),
                Point::new(list_x + 4, row_mid),
                TEXT_BOLD_WHITE,
                left_style,
            )
            .draw(display)?;
        } else if i == pos && !available {
            // Selected but on cooldown: outline only, black text on
            // white.  The dim pass below halftones both into grey so
            // the row reads as "selected but locked out".
            Rectangle::new(
                Point::new(inner_x, row_top),
                Size::new(inner_w, ITEM_H as u32),
            )
            .into_styled(PrimitiveStyle::with_stroke(BLACK, 1))
            .draw(display)?;
            Text::with_text_style(
                display_label.as_str(),
                Point::new(list_x + 4, row_mid),
                TEXT_BOLD_BLACK,
                left_style,
            )
            .draw(display)?;
        } else {
            // Not selected.
            Text::with_text_style(
                display_label.as_str(),
                Point::new(list_x + 4, row_mid),
                TEXT_BOLD_BLACK,
                left_style,
            )
            .draw(display)?;
        }

        // Apply the grey-out overlay last so it covers everything we
        // just drew for this row.
        if !available {
            dim_rect(
                display,
                Rectangle::new(
                    Point::new(inner_x, row_top),
                    Size::new(inner_w, ITEM_H as u32),
                ),
            )?;
        }
    }

    // ── Scroll arrows ─────────────────────────────────────────────────
    // Drawn after the rows so they overlay correctly.  Each is a
    // filled triangle horizontally centred in `inner_w`, occupying
    // most of the `ARROW_H`-tall padding strip with a 1 px gap on
    // both sides for breathing room.
    if needs_scroll {
        let cx = inner_x + inner_w as i32 / 2;
        let half_w: i32 = 7;
        if scroll_top > 0 {
            // Up arrow: apex at top, base at bottom.
            let top_y = list_y + 1;
            let bot_y = list_y + ARROW_H - 2;
            Triangle::new(
                Point::new(cx, top_y),
                Point::new(cx - half_w, bot_y),
                Point::new(cx + half_w, bot_y),
            )
            .into_styled(PrimitiveStyle::with_fill(BLACK))
            .draw(display)?;
        }
        if scroll_top + visible_rows < items.len() {
            // Down arrow: base at top, apex at bottom.
            let top_y = list_bottom - ARROW_H + 2;
            let bot_y = list_bottom - 1;
            Triangle::new(
                Point::new(cx - half_w, top_y),
                Point::new(cx + half_w, top_y),
                Point::new(cx, bot_y),
            )
            .into_styled(PrimitiveStyle::with_fill(BLACK))
            .draw(display)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Disabled-state dimming
// ---------------------------------------------------------------------------

/// Overdraw a rectangle with a 1-in-3 black diagonal stripe pattern,
/// darkening the white background of disabled menu rows so they read
/// as "unavailable" on a low-contrast EPD.  Black text on top stays
/// black; the surrounding background drops to ~33 % black.
fn dim_rect<D>(display: &mut D, rect: Rectangle) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let x0 = rect.top_left.x;
    let y0 = rect.top_left.y;
    let x1 = x0 + rect.size.width as i32;
    let y1 = y0 + rect.size.height as i32;
    let pixels = (y0..y1).flat_map(move |y| {
        (x0..x1).filter_map(move |x| {
            if (x + y).rem_euclid(3) == 0 {
                Some(Pixel(Point::new(x, y), BLACK))
            } else {
                None
            }
        })
    });
    display.draw_iter(pixels)
}

// ---------------------------------------------------------------------------
// Stats view — 5 labeled bars showing pet health at a glance
// ---------------------------------------------------------------------------

/// Bar width in pixels (modal inner is 128 px; subtract label width + margins).
const BAR_MAX_W: u32 = 60;
/// Bar height — the documented minimum for the 13px inline percentage
/// font to fit without clipping.  Was 16 with 5 bars; had to shrink to
/// this floor to fit 6 without the last bar (`Fit`) colliding with the
/// footer text drawn below (that collision was a real bug — see the
/// vertical budget note below).
const BAR_H: u32 = 15;
/// Vertical spacing between bars (== BAR_H, zero extra gap). Six bars
/// at 15px each, starting 2px below the title bar, land exactly on the
/// same bottom edge (y=122) the original 5-bar/18px layout used —
/// same proven clearance above the footer, just spread across one more
/// row. The previous attempt kept BAR_H/BAR_SPACING at 16 with only a
/// 2px reduction elsewhere, which undershot: the last bar ended up 8px
/// into the footer's text, rendering black-on-white footer glyphs over
/// the bar (looked like the bar had "no background").
const BAR_SPACING: i32 = 15;

fn draw_stats_view<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    use super::lifecycle;
    use super::stat_bar::draw_stat_bar;
    use crate::RED;

    super::note_live_view_drawn();

    // Get fresh stats (triggers an update if needed).
    let stats = match lifecycle::cycle() {
        Some(s) => s,
        None => return Ok(()),
    };

    // White popover frame + black border.
    ui::draw_popover_frame(
        display,
        Point::new(MARGIN, MARGIN),
        Size::new(MODAL_W, MODAL_H),
        BORDER,
    )?;

    // Title bar.
    let inner_x = MARGIN + BORDER as i32;
    let inner_y = MARGIN + BORDER as i32;
    let inner_w = MODAL_W - BORDER * 2;
    ui::draw_title_bar(display, "Pet Stats", Point::new(inner_x, inner_y), inner_w)?;

    // Stat bars.
    let bars: [(&str, u8); 5] = [
        ("Hunger", stats.hunger),
        ("Rested", stats.tired),
        ("Healthy", stats.healthy),
        ("Happy", stats.happy),
        ("Fit", stats.fit),
    ];

    // Layout: label at left margin, bar to the right of the longest
    // label ("Healthy" = 7 chars × 7 px = 49 px from `label_x`).
    let label_x = inner_x + 4;
    let bar_x = label_x + 60; // 4 px clearance after the longest label

    for (i, (label, value)) in bars.iter().enumerate() {
        let y = inner_y + TITLE_H + 2 + i as i32 * BAR_SPACING;
        // Label vertically centred against the bar.
        let label_y = y + (BAR_H as i32 - 13) / 2;
        // Critical (< 25 %) values are highlighted in red.
        let fill_color = if *value < 25 { RED } else { BLACK };

        draw_stat_bar(
            display,
            label,
            *value,
            Point::new(label_x, label_y),
            Point::new(bar_x, y),
            Size::new(BAR_MAX_W, BAR_H),
            fill_color,
        )?;
    }

    // Footer: name, generation + age.
    let name = super::lifecycle::pet_name();
    let age_hours = stats.age_ticks / 360;
    let age_days = age_hours / 24;
    // Footer baseline; moved down 8 px so it no longer collides with
    // the bottom of the last stat bar (which now occupies y ≤ 122).
    let footer_y = MARGIN + MODAL_H as i32 - BORDER as i32 - 6;
    let mut footer: heapless::String<32> = heapless::String::new();
    // `*` after the name when BPETS_V3.CFG overrode at least one
    // threshold field — lets the player tell at a glance that their
    // pet is running on a non-standard balance.
    let custom_mark = if super::engine::thresholds::is_custom() {
        "*"
    } else {
        ""
    };
    if !name.is_empty() {
        let _ = core::fmt::Write::write_fmt(
            &mut footer,
            format_args!(
                "{}{} | {}d {}h",
                name,
                custom_mark,
                age_days,
                age_hours % 24
            ),
        );
    } else {
        let _ = core::fmt::Write::write_fmt(
            &mut footer,
            format_args!(
                "Gen {}{} | {}d {}h",
                stats.generation,
                custom_mark,
                age_days,
                age_hours % 24
            ),
        );
    }
    crate::ui::draw_text(
        display,
        footer.as_str(),
        Point::new(MARGIN + MODAL_W as i32 / 2, footer_y),
        TEXT_BOLD_BLACK,
        crate::ui::CENTERED_BOTTOM,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Item, ModalKind, cooldown_string, first_selectable, step_cursor};
    use crate::game::engine::{GameState, PetKind, Phase};
    use crate::game::lifecycle;

    fn active_pet() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        s
    }

    #[test]
    fn countdowns_are_readable_up_to_three_days() {
        assert_eq!(cooldown_string(0), None);
        assert_eq!(cooldown_string(5).as_deref(), Some(" (50s)"));
        assert_eq!(cooldown_string(59 * 6).as_deref(), Some(" (59m)"));
        assert_eq!(cooldown_string(2 * 360).as_deref(), Some(" (2h00)"));
        assert_eq!(cooldown_string(2 * 360 + 30).as_deref(), Some(" (2h05)"));
        // Rehab: 4320 minutes = 72 h = 25920 ticks (x10 s overflowed u16).
        assert_eq!(cooldown_string(25920).as_deref(), Some(" (72h00)"));
    }

    #[test]
    fn rest_menu_offers_sleep_relax_wash_wake() {
        let items = ModalKind::Rest.items();
        for want in [Item::Sleep, Item::Relax, Item::Wash, Item::WakeFromSleep, Item::Cancel] {
            assert!(items.contains(&want), "missing {}", want.label());
        }
        assert_ne!(Item::WakeFromSleep.label(), Item::WakeUp.label());

        let mut s = active_pet();
        let awake = s.stats(0);
        assert!(Item::Sleep.available(&awake));
        assert!(!Item::WakeFromSleep.available(&awake));
        assert!(s.sleep());
        let asleep = s.stats(0);
        assert!(!Item::Sleep.available(&asleep));
        assert!(Item::WakeFromSleep.available(&asleep));
    }

    #[test]
    fn wash_row_follows_the_engine_refusal() {
        let _guard = crate::game::lock_test_state();
        use crate::game::engine::doses::{Reason, Refusal};
        lifecycle::start_new_game(PetKind(0), false, false);
        lifecycle::with_state(|s| {
            s.phase = Phase::Active;
            s.dirt = 0;
            true
        });
        let clean = lifecycle::cycle().unwrap();
        assert!(!Item::Wash.available(&clean));
        assert!(matches!(Item::Wash.reason(&clean), Reason::Refused(Refusal::AlreadyClean)));
        assert_eq!(Item::Wash.cooldown_ticks(&clean), 0);
        lifecycle::with_state(|s| {
            s.dirt = u16::MAX / 2;
            true
        });
        assert!(Item::Wash.available(&lifecycle::cycle().unwrap()));
    }

    #[test]
    fn sleep_row_says_busy_while_an_action_runs() {
        let _guard = crate::game::lock_test_state();
        use crate::game::engine::doses::Reason;
        lifecycle::start_new_game(PetKind(0), false, false);
        lifecycle::with_state(|s| {
            s.phase = Phase::Active;
            true
        });
        assert!(Item::Sleep.available(&lifecycle::cycle().unwrap()));
        lifecycle::with_state(|s| {
            s.dirt = u16::MAX / 2;
            true
        });
        assert!(lifecycle::wash());
        let busy = lifecycle::cycle().unwrap();
        assert!(!Item::Sleep.available(&busy));
        assert!(matches!(Item::Sleep.reason(&busy), Reason::Busy));
        assert!(!lifecycle::sleep());
    }

    #[test]
    fn debug_page_is_the_v3_table() {
        let want = [
            "Engine info", "Skip 10 min", "Skip 1 hour", "Skip 1 day", "Comfort", "Hungry",
            "Thirsty", "Stuffed", "Tired", "Bored", "Dirty", "Fat", "Thin", "Unfit", "Drunk",
            "Hung over", "Groggy", "Injured", "Diabetic", "Dependent", "Clear conditions",
            "Clear cooldowns", "Reset battle record",
        ];
        let items = ModalKind::Debug.items();
        let labels: std::vec::Vec<&str> = items.iter().map(|i| i.label()).collect();
        assert_eq!(labels[..want.len()], want);
        assert_eq!(labels.len(), want.len() + 1);
        assert!(items[want.len()] == Item::Cancel);
    }

    #[test]
    fn a_refused_poke_says_why() {
        use crate::game::engine::DebugPoke;
        use crate::game::engine::doses::Reason;
        let _guard = crate::game::lock_test_state();
        let hungry = ModalKind::Debug
            .items()
            .iter()
            .copied()
            .find(|i| i.label() == "Hungry")
            .unwrap();
        // A fresh egg is still hatching.
        lifecycle::start_new_game(PetKind(0), false, false);
        assert_eq!(lifecycle::debug_poke(DebugPoke::Hungry), Err(Reason::Egg));
        hungry.activate();
        assert_eq!(crate::game::last_reason_for_test(), Reason::Egg);
        lifecycle::with_state(|s| {
            s.phase = Phase::Gone;
            true
        });
        assert_eq!(lifecycle::debug_poke(DebugPoke::Hungry), Err(Reason::Gone));
        hungry.activate();
        assert_eq!(crate::game::last_reason_for_test(), Reason::Gone);
        lifecycle::with_state(|s| {
            s.phase = Phase::Active;
            s.hibernating = true;
            true
        });
        assert_eq!(lifecycle::debug_poke(DebugPoke::Hungry), Err(Reason::Hibernating));
        hungry.activate();
        assert_eq!(crate::game::last_reason_for_test(), Reason::Hibernating);
        super::close();
    }

    #[test]
    fn a_day_through_the_menu() {
        let _guard = crate::game::lock_test_state();
        lifecycle::start_new_game(PetKind(0), false, false);
        lifecycle::with_state(|s| {
            s.phase = Phase::Active;
            s.fullness = u16::MAX / 4;
            s.hydration = u16::MAX / 4;
            s.dirt = u16::MAX / 2;
            s.energy = u16::MAX / 4;
            true
        });
        let st = || lifecycle::cycle().unwrap();
        // Take a menu row, then let its timed action finish.
        let ok = |item: Item| {
            let stats = st();
            assert!(item.available(&stats), "{} unavailable", item.label());
            item.activate();
            super::close();
            lifecycle::debug_skip_minutes(5);
        };
        let before = st();
        ok(Item::FeedFood(crate::game::engine::FoodKind::Apple));
        assert!(st().hunger > before.hunger);
        ok(Item::DrinkChoice(crate::game::engine::DrinkKind::Water));
        assert!(st().water > before.water);
        ok(Item::Wash);
        assert!(st().clean > before.clean);
        ok(Item::Sleep);
        assert!(st().is_sleeping);
        lifecycle::debug_skip_minutes(60);
        let slept = st();
        assert!(slept.tired > before.tired);
        ok(Item::WakeFromSleep);
        assert!(!st().is_sleeping);
        let before_relax = st();
        Item::Relax.activate();
        super::close();
        assert_eq!(st().active_action, Some(crate::game::engine::Action::Relax));
        lifecycle::debug_skip_minutes(5);
        let after_relax = st();
        assert!(after_relax.active_action.is_none());
        assert!(after_relax.tired > before_relax.tired, "relax restores energy");
    }

    /// Rows 0 and 2 disabled, like a Play list with Battle and a
    /// paid game locked out.
    fn sel(i: usize) -> bool {
        !matches!(i, 0 | 2)
    }

    #[test]
    fn open_skips_disabled_first_row() {
        assert_eq!(first_selectable(5, sel), 1);
        assert_eq!(first_selectable(5, |_| true), 0);
        assert_eq!(first_selectable(5, |_| false), 0);
        assert_eq!(first_selectable(0, |_| true), 0);
    }

    #[test]
    fn the_cursor_steps_one_row_and_wraps() {
        assert_eq!(step_cursor(5, 1, true), 2);
        assert_eq!(step_cursor(5, 4, true), 0);
        assert_eq!(step_cursor(5, 2, false), 1);
        assert_eq!(step_cursor(5, 0, false), 4);
        assert_eq!(step_cursor(1, 0, true), 0);
        assert_eq!(step_cursor(0, 0, false), 0);
    }
}
