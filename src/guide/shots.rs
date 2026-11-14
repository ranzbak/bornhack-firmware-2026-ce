//! The booklet's screen shots (`guide shot NAME`). A shot is a list of steps
//! played on a fresh game against a fake clock, and an expectation that
//! checks the game is in the state the shot claims to show. A shot whose
//! expectation fails is an error, so a picture can never show something else
//! than its name says.
//!
//! The game's state is global: run one shot per process.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use crate::game::engine::sim;
use crate::game::engine::status::{Condition, Word};
use crate::game::engine::thresholds::{HATCHING_TICKS, Mode, badge_preset};
use crate::game::engine::{DebugPoke, DisplayAnim, FoodKind, PetKind};
use crate::game::modal::{self, ModalKind};
use crate::game::stats_pages::Page;
use crate::game::{Toast, current_toast, diary_view, lifecycle, pet_select};
use crate::menu::{ButtonId, dispatch_press};
use crate::platform::{fake_uptime_ms, set_fake_uptime_ms};
use crate::sim_support;
use crate::text_entry;

/// The pet's name in every shot (the hero is `PetKind::roster()[0]`).
pub const HERO_NAME: &str = "Bob";

/// One game tick on the fake clock, in milliseconds.
const TICK_MS: u64 = 10_000;
/// Most half-hour skips a shot waits for the pet to leave, then to be gone.
const MAX_SKIPS: u32 = 1000;
/// Length of one waiting skip, in minutes.
const SKIP_MIN: u32 = 30;
/// Most pizzas a shot feeds before a cause shows.
const MAX_PIZZAS: u32 = 12;

#[derive(Clone, Copy)]
pub enum Step {
    Press(ButtonId),
    Start {
        money: bool,
        hard: bool,
    },
    Poke(DebugPoke),
    Skip(u32),
    Ticks(u32),
    Open(ModalKind),
    /// A loop or a shared set-up: the steps that cannot be a plain list.
    Do(fn()),
}

pub struct Shot {
    pub name: &'static str,
    pub steps: &'static [Step],
    pub expect: fn() -> bool,
}

pub struct Summary {
    pub w: u32,
    pub h: u32,
    pub non_white: usize,
}

use Step::{Do, Open, Poke, Press, Start};

// ── Shared set-ups ──────────────────────────────────────────────────────────

fn advance_ticks(n: u32) {
    set_fake_uptime_ms(fake_uptime_ms().unwrap_or(0) + n as u64 * TICK_MS);
}

fn play(steps: &[Step]) {
    for step in steps {
        match *step {
            Press(b) => dispatch_press(b),
            Start { money, hard } => {
                lifecycle::start_new_game(PetKind::roster()[0], money, hard);
                lifecycle::set_pet_name(HERO_NAME.as_bytes());
            }
            Poke(p) => lifecycle::debug_poke(p)
                .unwrap_or_else(|why| panic!("poke refused: {}", why.text())),
            Step::Skip(m) => lifecycle::debug_skip_minutes(m),
            Step::Ticks(n) => advance_ticks(n),
            Open(k) => modal::open(k),
            Do(f) => f(),
        }
        let _ = lifecycle::cycle();
    }
}

/// Wait out the egg: its countdown, and one tick more.
fn hatch() {
    advance_ticks(HATCHING_TICKS() as u32 + 1);
}

static HATCHED: &[Step] = &[
    Start {
        money: true,
        hard: false,
    },
    Do(hatch),
];
static CONTENT: &[Step] = &[Do(hatched), Poke(DebugPoke::Comfort)];
static STARVING: &[Step] = &[Do(content), Poke(DebugPoke::Hungry)];
static STATS_NEEDS: &[Step] = &[Do(starving), Open(ModalKind::Stats), Press(ButtonId::Fire)];
static CAUSE_TOAST: &[Step] = &[Do(content), Do(provoke_cause)];
static LEAVING: &[Step] = &[
    Do(content),
    Poke(DebugPoke::Hungry),
    Poke(DebugPoke::Thirsty),
    Do(wait_for_leaving),
];
static DEBUG_MENU: &[Step] = &[
    Do(content),
    Press(ButtonId::Up),
    Press(ButtonId::Up),
    Press(ButtonId::Down),
    Press(ButtonId::Down),
    Press(ButtonId::Left),
    Press(ButtonId::Right),
    Press(ButtonId::Left),
    Press(ButtonId::Right),
    Press(ButtonId::Fire),
];

fn hatched() {
    play(HATCHED);
}
fn content() {
    play(CONTENT);
}
fn starving() {
    play(STARVING);
}
fn stats_needs() {
    play(STATS_NEEDS);
}
fn cause_toast() {
    play(CAUSE_TOAST);
}
fn leaving() {
    play(LEAVING);
}
fn debug_menu() {
    play(DEBUG_MENU);
}

/// What the firmware's render loop does once the egg has hatched: ask for a
/// name, with a default filled in.
fn name_prompt() {
    assert!(lifecycle::take_naming_pending(), "the egg has not hatched");
    let default = lifecycle::random_default_name(lifecycle::now_tick());
    text_entry::begin(
        default.as_bytes(),
        12,
        |name| lifecycle::set_pet_name(name),
        "Name your Pet",
        true,
    );
}

fn sleep() {
    assert!(lifecycle::sleep(), "the pet would not go to sleep");
}

fn hibernate() {
    assert!(lifecycle::hibernate(), "the pet would not hibernate");
}

/// Feed pizza until the pet is told why it feels bad (a cause toast).
fn provoke_cause() {
    for _ in 0..MAX_PIZZAS {
        if current_toast() == Some(Toast::Cause) {
            return;
        }
        lifecycle::feed(FoodKind::Pizza);
        // Let the meal finish, then look.
        advance_ticks(10);
        let _ = lifecycle::cycle();
    }
    assert!(toast_is(Toast::Cause), "no cause ever showed");
}

fn skip_until(done: fn() -> bool) {
    for _ in 0..MAX_SKIPS {
        if done() {
            return;
        }
        lifecycle::debug_skip_minutes(SKIP_MIN);
        let _ = lifecycle::cycle();
    }
    assert!(done(), "the wait for the pet never ended");
}

fn is_leaving() -> bool {
    matches!(lifecycle::display_anim(), DisplayAnim::Leaving { .. })
}

fn is_gone() -> bool {
    lifecycle::display_anim() == DisplayAnim::Gone
}

fn wait_for_leaving() {
    skip_until(is_leaving);
}

fn wait_for_gone() {
    skip_until(is_gone);
}

static GONE: &[Step] = &[Do(leaving), Do(wait_for_gone)];

fn gone() {
    play(GONE);
}

// ── Expectations ────────────────────────────────────────────────────────────

fn line() -> Option<crate::game::engine::status::StatusLine> {
    lifecycle::status_line()
}

fn first_is(w: Word) -> bool {
    line().is_some_and(|l| l.first == w)
}

fn has_word(w: Word) -> bool {
    line().is_some_and(|l| l.first == w || l.second == Some(w))
}

fn has_condition(c: Condition) -> bool {
    lifecycle::cycle().is_some_and(|s| s.conditions.has(c))
}

fn menu_is(kind: ModalKind) -> bool {
    modal::open_kind() == kind
}

fn toast_is(t: Toast) -> bool {
    current_toast() == Some(t)
}

macro_rules! shot {
    ($name:expr, [$($step:expr),* $(,)?], $expect:expr) => {
        Shot { name: $name, steps: &[$($step),*], expect: $expect }
    };
}

use ButtonId::{Down, Fire, Right};

pub static SHOTS: &[Shot] = &[
    shot!("start", [], || !lifecycle::is_started()),
    shot!("select-pet", [Press(Fire)], pet_select::is_active),
    shot!("select-mode", [Press(Fire), Press(Fire)], || {
        pet_select::is_active() && pet_select::phase() == pet_select::Phase::Money
    }),
    // The badge asks for the name once the egg has hatched (the firmware's
    // render loop does it; there is none here), so the shot plays that too.
    shot!(
        "naming",
        [
            Press(Fire),
            Press(Fire),
            Press(Fire),
            Do(hatch),
            Do(name_prompt)
        ],
        text_entry::is_active
    ),
    shot!(
        "egg",
        [Start {
            money: true,
            hard: false
        }],
        || {
            matches!(
                lifecycle::display_anim(),
                DisplayAnim::Hatching { ticks_remaining } if ticks_remaining > 1
            )
        }
    ),
    shot!("hatched", [Do(hatched)], || {
        lifecycle::is_started()
            && !matches!(lifecycle::display_anim(), DisplayAnim::Hatching { .. })
    }),
    shot!("content", [Do(content)], || first_is(Word::Content)),
    shot!("starving", [Do(starving)], || {
        line().is_some_and(|l| l.first == Word::Starving && l.danger)
    }),
    shot!(
        "two-words",
        [Do(starving), Poke(DebugPoke::Thirsty)],
        || line().is_some_and(|l| l.second.is_some())
    ),
    shot!(
        "asleep",
        [Do(content), Poke(DebugPoke::Tired), Do(sleep)],
        || { first_is(Word::Asleep) }
    ),
    shot!(
        "hung-over",
        [Do(content), Poke(DebugPoke::HungOver)],
        || { has_condition(Condition::HungOver) }
    ),
    shot!("injured", [Do(content), Poke(DebugPoke::Injured)], || {
        first_is(Word::Injured)
    }),
    shot!("wasted", [Do(content), Poke(DebugPoke::Drunk)], || {
        line().is_some_and(|l| l.first == Word::Wasted && l.danger)
    }),
    shot!(
        "no-insulin",
        [Do(content), Poke(DebugPoke::Diabetic)],
        || { has_word(Word::NoInsulin) }
    ),
    shot!(
        "needs-rehab",
        [Do(content), Poke(DebugPoke::Dependent)],
        || { has_word(Word::NeedsRehab) }
    ),
    shot!("stats-needs", [Do(stats_needs)], || {
        modal::stats_view_open() && modal::stats_page() == Page::Needs
    }),
    shot!("stats-body", [Do(stats_needs), Press(Right)], || {
        modal::stats_view_open() && modal::stats_page() == Page::Body
    }),
    shot!(
        "hint",
        [
            Do(starving),
            Open(ModalKind::Stats),
            Press(Down),
            Press(Fire)
        ],
        modal::hint_view_open
    ),
    shot!("cause-toast", [Do(cause_toast)], || toast_is(Toast::Cause)),
    shot!(
        "diary",
        [
            Do(cause_toast),
            Open(ModalKind::Stats),
            Press(Down),
            Press(Down),
            Press(Fire)
        ],
        diary_view::is_active
    ),
    shot!("menu-feed", [Do(content), Open(ModalKind::Feed)], || {
        menu_is(ModalKind::Feed)
    }),
    shot!("menu-drink", [Do(content), Open(ModalKind::Drink)], || {
        menu_is(ModalKind::Drink)
    }),
    shot!("menu-rest", [Do(content), Open(ModalKind::Rest)], || {
        menu_is(ModalKind::Rest)
    }),
    shot!("menu-heal", [Do(content), Open(ModalKind::Heal)], || {
        menu_is(ModalKind::Heal)
    }),
    shot!("menu-play", [Do(content), Open(ModalKind::Play)], || {
        menu_is(ModalKind::Play)
    }),
    shot!("menu-stats", [Do(content), Open(ModalKind::Stats)], || {
        menu_is(ModalKind::Stats)
    }),
    shot!(
        "menu-feed-hard",
        [
            Start {
                money: true,
                hard: true
            },
            Do(hatch),
            Poke(DebugPoke::Comfort),
            Open(ModalKind::Feed)
        ],
        || menu_is(ModalKind::Feed) && lifecycle::hard_mode()
    ),
    shot!(
        "risk-marker",
        [Do(content), Poke(DebugPoke::Stuffed), Open(ModalKind::Feed)],
        || menu_is(ModalKind::Feed) && modal::risk_row_count() > 0
    ),
    shot!(
        "refusal-water",
        [
            Do(content),
            Open(ModalKind::Drink),
            Press(Fire),
            Step::Ticks(3),
            Open(ModalKind::Drink),
            Press(ButtonId::Up),
            Press(Fire)
        ],
        || toast_is(Toast::Reason)
    ),
    shot!(
        "hibernating",
        [Do(content), Do(hibernate)],
        lifecycle::is_hibernating
    ),
    shot!("leaving", [Do(leaving)], is_leaving),
    shot!("gone", [Do(gone)], is_gone),
    shot!("debug-menu", [Do(debug_menu)], || menu_is(ModalKind::Debug)),
    shot!("engine-info", [Do(debug_menu), Press(Fire)], || {
        toast_is(Toast::DebugInfo)
    }),
];

/// Play the shot `name` on a fresh game, check its expectation, save
/// `<out_dir>/<name>.png`. One shot per process.
pub fn run(name: &str, out_dir: &Path) -> Result<Summary, String> {
    let shot = SHOTS
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("no shot named {name}"))?;
    set_fake_uptime_ms(0);
    // The badge's numbers, never the bare simulator preset.
    sim::install_preset(badge_preset(Mode::Classic), Mode::Classic, false);
    sim_support::install_pets_from_repo_cfg();

    catch_unwind(AssertUnwindSafe(|| play(shot.steps))).map_err(|e| {
        let why = e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        format!("shot {name}: a step failed: {why}")
    })?;
    if !(shot.expect)() {
        return Err(format!(
            "shot {name}: the game is not in the state the shot shows (status line {:?}, anim {:?})",
            lifecycle::status_line(),
            lifecycle::display_anim(),
        ));
    }

    let display = sim_support::render();
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    sim_support::save_png(&display, &out_dir.join(format!("{name}.png")))?;
    let size = embedded_graphics::prelude::OriginDimensions::size(&display);
    Ok(Summary {
        w: size.width * sim_support::PNG_SCALE,
        h: size.height * sim_support::PNG_SCALE,
        non_white: sim_support::non_white(&display),
    })
}
