//! BornPets v3 simulator: a minute-stepping oracle and a delta driver that
//! play the same routine, ported from `prototypes/bornpets-simv3`.
//!
//! Host-only (`simulator` feature); nothing here is linked into firmware.
//! The oracle exists only to prove that `update()` (the delta engine) gives
//! the same pet as stepping one minute at a time. The badge never loops.

use std::cell::Cell;

use super::needs::P;
use super::thresholds::{self, Mode, Thresholds};
use super::{DrinkKind, FoodKind, GameState, MiniGame, PetKind, Phase};

/// Engine ticks per minute (1 tick = 10 s).
const TPM: u32 = 6;
/// Ticks in a simulated day.
pub const TICKS_PER_DAY: u32 = 1440 * TPM;
/// Seed of `run_routine`; it only moves the pet's three traits.
pub const DEFAULT_SEED: u64 = 42;
/// Minute of day 0 at which the pet starts (08:00).
const START_MINUTE: u32 = 480;
/// One percent of a level, as an `i32`.
const PCT: i32 = P;

/// How the clock reaches the next routine event.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Driver {
    /// `update()` once per minute: every segment is a one-minute `advance`.
    Oracle,
    /// Like the firmware: `update()` only at routine events, at the engine's
    /// own `next_wake_tick`, and at each day end (a sampling point).
    Delta,
}

/// One player step of a routine. The engine's `Action` cannot carry a food,
/// a drink, sleep or a mini-game, so routines speak this instead; each maps
/// to one public `GameState` call and a refusal is never bypassed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    Wake,
    Sleep,
    Eat(FoodKind),
    Drink(DrinkKind),
    Play,
    /// Exercise.
    Sport,
    Wash,
    /// A mini-game of grade 1 (draw) or 2 (win).
    Mini(u8),
    /// Meeting a new mesh friend (stimulation +15).
    Friend,
    Insulin,
}

/// What a routine returns for one minute: at most eight steps.
pub type Plan = heapless::Vec<Act, 8>;

/// Result of one run. Percentages are whole percent of the level.
#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub alive: bool,
    /// End phase: a pet at health 0 shows as `Leaving` while still `alive`.
    pub phase: Phase,
    pub days: f32,
    pub avg_mood_pct: u8,
    pub end_health_pct: u8,
    pub min_health_pct: u8,
    pub end_fitness_pct: u8,
    pub end_weight_pct: u8,
    pub injuries: u16,
    /// Delta driver only (the oracle has no engine wakes): wake-ups that came
    /// from `next_wake_tick`, not from routine events or the day-end sample.
    pub engine_wakes_per_day: f32,
}

/// An `Outcome` plus the cost of the `next_wake_tick` calls the delta
/// driver made (`ADVANCE_CALLS` per call).
#[derive(Clone, Copy, Debug)]
pub struct Run {
    pub outcome: Outcome,
    pub wake_queries: u32,
    pub wake_calls_total: u64,
    pub wake_calls_max: u32,
}

// ---------------------------------------------------------------------------
// Probe: mood and health integrated by `update`, so the delta driver sees the
// same per-minute figures as the oracle without stepping.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Probe {
    /// Sum over minutes of (mood before + mood after), i.e. twice the integral.
    mood_sum2: i64,
    minutes: u64,
    min_health: u16,
}

const PROBE_NEW: Probe = Probe {
    mood_sum2: 0,
    minutes: 0,
    min_health: u16::MAX,
};

thread_local! {
    static PROBE: Cell<Probe> = const { Cell::new(PROBE_NEW) };
}

/// Called by `GameState::update` after each closed-form segment of `dt`
/// minutes: mood and health are linear inside one, so ends are enough.
pub(super) fn note_segment(
    mood_before: u16,
    mood_after: u16,
    health_before: u16,
    health_after: u16,
    dt: u32,
) {
    PROBE.with(|p| {
        let mut v = p.get();
        v.mood_sum2 += (mood_before as i64 + mood_after as i64) * dt as i64;
        v.minutes += dt as u64;
        v.min_health = v.min_health.min(health_before).min(health_after);
        p.set(v);
    });
}

// ---------------------------------------------------------------------------
// Threshold install (R14)
// ---------------------------------------------------------------------------

/// Install a preset (or custom `values`) for this process. The engine's
/// table is write-once, so this takes effect only on the first call and only
/// before any pet runs: call it first thing in `main` or in a test binary of
/// its own. Returns whether this call was the one that installed.
pub fn install_preset(values: Thresholds, mode: Mode, custom: bool) -> bool {
    static ONCE: std::sync::Once = std::sync::Once::new();
    let mut first = false;
    ONCE.call_once(|| {
        thresholds::install(values, mode, custom);
        first = true;
    });
    first
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

type Routine<'a> = &'a dyn Fn(u32, u32, &GameState) -> Plan;

/// Run `routine` for `days` days against a fresh egg (`DEFAULT_SEED`).
pub fn run_routine(routine: Routine, days: u32, driver: Driver) -> Outcome {
    run_full(routine, days, driver, DEFAULT_SEED).outcome
}

/// Does the routine have an event at this minute, whatever the pet's state?
struct Probes {
    lo: GameState,
    hi: GameState,
    mid: GameState,
}

impl Probes {
    fn new() -> Self {
        let mut lo = GameState::new_egg(0, PetKind::DEFAULT);
        lo.fullness = 0;
        lo.hydration = 0;
        lo.energy = u16::MAX;
        lo.fun = 0;
        lo.dirt = u16::MAX;
        lo.strain = 0;
        lo.booze = 0;
        lo.health = 0;
        lo.fitness = 0;
        let mut hi = GameState::new_egg(0, PetKind::DEFAULT);
        hi.fullness = u16::MAX;
        hi.hydration = u16::MAX;
        hi.dirt = 0;
        Probes {
            lo,
            hi,
            mid: GameState::new_egg(0, PetKind::DEFAULT),
        }
    }

    fn fires(&self, r: Routine, d: u32, m: u32) -> bool {
        !r(d, m, &self.lo).is_empty()
            || !r(d, m, &self.hi).is_empty()
            || !r(d, m, &self.mid).is_empty()
    }
}

/// Start one step. `true` when it started (a timed action) or happened.
fn perform(s: &mut GameState, a: Act) -> bool {
    match a {
        Act::Wake => s.wake(),
        Act::Sleep => s.sleep(),
        Act::Eat(f) => s.feed(f),
        Act::Drink(d) => s.drink(d),
        Act::Play => s.play(),
        Act::Sport => s.exercise(),
        Act::Wash => s.wash(),
        Act::Insulin => s.medicate(),
        Act::Friend => {
            s.friend_boost(true);
            true
        }
        Act::Mini(g) => {
            if s.game_refusal(Some(MiniGame::TicTacToe)).is_some() {
                return false;
            }
            s.award_inspiration(MiniGame::TicTacToe, g.min(2));
            true
        }
    }
}

/// Play every step of a plan in order. A timed action runs to its end (the
/// action ticks age no need) before the next step; a refused step is skipped.
fn play_plan(s: &mut GameState, plan: &Plan, injuries: &mut u16) {
    for &a in plan.iter() {
        if s.phase == Phase::Gone {
            return;
        }
        if !perform(s, a) {
            continue;
        }
        if s.active_action.is_some() {
            let was_injured = s.injured_min > 0;
            s.update(s.last_update_tick + s.action_ticks_remaining as u32);
            if !was_injured && s.injured_min > 0 {
                *injuries += 1;
            }
        }
    }
}

/// `run_routine` with a seed, plus the wake-cost figures.
pub fn run_full(routine: Routine, days: u32, driver: Driver, seed: u64) -> Run {
    PROBE.with(|p| p.set(PROBE_NEW));
    let probes = Probes::new();
    let mut s = GameState::new_egg(seed, PetKind::DEFAULT);
    // The routines never earn HAX; with the gate on they would starve.
    s.money_enabled = false;
    // The egg hatches and the pet is fresh at 08:00 on day 0, as in the
    // prototype (hatching ticks past the countdown age nothing).
    s.update(START_MINUTE * TPM);

    let (mut injuries, mut wakes) = (0u16, 0u32);
    let (mut queries, mut calls_total, mut calls_max) = (0u32, 0u64, 0u32);

    'days: for d in 0..days {
        let first = if d == 0 { START_MINUTE } else { 0 };
        let day_end = (d + 1) * TICKS_PER_DAY;
        match driver {
            Driver::Oracle => {
                for m in first..1440 {
                    s.update(d * TICKS_PER_DAY + m * TPM);
                    if s.phase == Phase::Gone {
                        break 'days;
                    }
                    if probes.fires(routine, d, m) {
                        let plan = routine(d, m, &s);
                        play_plan(&mut s, &plan, &mut injuries);
                    }
                }
                s.update(day_end);
            }
            Driver::Delta => {
                let mut m = first;
                loop {
                    // The next routine event today, or the day end.
                    while m < 1440 && !probes.fires(routine, d, m) {
                        m += 1;
                    }
                    let at = if m < 1440 {
                        d * TICKS_PER_DAY + m * TPM
                    } else {
                        day_end
                    };
                    while s.last_update_tick < at && s.phase != Phase::Gone {
                        let before = super::ADVANCE_CALLS.with(|c| c.get());
                        let wake = s.next_wake_tick();
                        let cost = super::ADVANCE_CALLS.with(|c| c.get()) - before;
                        queries += 1;
                        calls_total += cost as u64;
                        calls_max = calls_max.max(cost);
                        if wake < at {
                            wakes += 1;
                        }
                        s.update(wake.min(at));
                    }
                    if m >= 1440 || s.phase == Phase::Gone {
                        break;
                    }
                    let plan = routine(d, m, &s);
                    play_plan(&mut s, &plan, &mut injuries);
                    m += 1;
                }
            }
        }
        if s.phase == Phase::Gone {
            break 'days;
        }
    }

    let probe = PROBE.with(|p| p.get());
    let days_run = (s.age_ticks.saturating_sub(START_MINUTE * TPM) as f32 / TICKS_PER_DAY as f32)
        .max(f32::MIN_POSITIVE);
    let pct = |v: u16| (v as i32 / PCT) as u8;
    let avg = if probe.minutes == 0 {
        0
    } else {
        (probe.mood_sum2 / (2 * probe.minutes as i64) / PCT as i64) as u8
    };
    Run {
        outcome: Outcome {
            alive: s.phase != Phase::Gone,
            phase: s.phase,
            days: s.age_ticks as f32 / TICKS_PER_DAY as f32,
            avg_mood_pct: avg,
            end_health_pct: pct(s.health),
            min_health_pct: pct(probe.min_health.min(s.health)),
            end_fitness_pct: pct(s.fitness),
            end_weight_pct: pct(s.weight),
            injuries,
            engine_wakes_per_day: wakes as f32 / days_run,
        },
        wake_queries: queries,
        wake_calls_total: calls_total,
        wake_calls_max: calls_max,
    }
}

// ---------------------------------------------------------------------------
// Routines (ported from prototypes/bornpets-simv3/src/main.rs)
// ---------------------------------------------------------------------------

const fn at(m: u32, h: u32, mi: u32) -> bool {
    m == h * 60 + mi
}

fn plan(acts: &[Act]) -> Plan {
    let mut v = Plan::new();
    for &a in acts.iter().take(Plan::new().capacity()) {
        let _ = v.push(a);
    }
    v
}

fn pct_lvl(p: i32) -> u16 {
    (p * PCT) as u16
}

/// The player's habits, applied to every event of a routine that has them:
/// water at a check-in (one under 55 % hydration, two under 25 %, before a
/// trailing Sleep), and with `weight_habit` richer food when thin and lighter
/// food when heavy (spec 15.8). A diabetic pet gets insulin first when the
/// last dose is nearly worn off. Anything past eight steps (never reached by the ported routines) is dropped.
fn habits(mut acts: Plan, s: &GameState, weight_habit: bool) -> Plan {
    if acts.is_empty() {
        return acts;
    }
    if acts.as_slice() != [Act::Sleep] && s.hydration < pct_lvl(55) {
        let w = if acts.last() == Some(&Act::Sleep) {
            acts.len() - 1
        } else {
            acts.len()
        };
        let n = if s.hydration < pct_lvl(25) { 2 } else { 1 };
        for _ in 0..n {
            let _ = acts.insert(w, Act::Drink(DrinkKind::Water));
        }
        // insert fails only when full; the extra water is then dropped
    }
    if weight_habit {
        for a in acts.iter_mut() {
            if let Act::Eat(f) = a {
                if s.weight < pct_lvl(42) && matches!(f, FoodKind::Salad | FoodKind::Apple) {
                    *f = FoodKind::Pizza;
                } else if s.weight > pct_lvl(60)
                    && matches!(f, FoodKind::Pizza | FoodKind::Frikandel)
                {
                    *f = FoodKind::Apple;
                } else if s.weight > pct_lvl(60) && *f == FoodKind::Cake {
                    *f = FoodKind::Salad;
                }
            }
        }
    }
    if s.diabetic && s.insulin_min as u32 <= thresholds::DOSE_INSULIN_WINDOW_MIN() as u32 {
        if acts.is_full() {
            acts.pop();
        }
        let _ = acts.insert(0, Act::Insulin);
    }
    acts
}

fn balanced_plan(m: u32, s: &GameState) -> Plan {
    use Act::*;
    use FoodKind::*;
    if at(m, 8, 0) {
        plan(&[Wake, Eat(Apple)])
    } else if at(m, 10, 30) {
        plan(&[Play])
    } else if at(m, 18, 0) {
        plan(&[Wash])
    } else if at(m, 13, 0) {
        plan(&[Eat(Apple)])
    } else if at(m, 15, 30) {
        plan(&[Sport])
    } else if at(m, 18, 30) {
        plan(&[Eat(Pizza)])
    } else if at(m, 20, 30) {
        plan(&[Mini(2), Drink(DrinkKind::Beer)])
    } else if at(m, 23, 30) {
        if s.fullness < pct_lvl(55) {
            plan(&[Eat(Salad), Sleep])
        } else {
            plan(&[Sleep])
        }
    } else {
        Plan::new()
    }
}

/// Three meals, a walk, a game, a beer and a decent bedtime.
pub fn balanced(_d: u32, m: u32, s: &GameState) -> Plan {
    habits(balanced_plan(m, s), s, true)
}

/// Two exercise sessions a day and lighter food.
pub fn sporty(_d: u32, m: u32, s: &GameState) -> Plan {
    use Act::*;
    use FoodKind::*;
    let p = if at(m, 7, 30) {
        plan(&[Wake, Eat(Apple), Eat(Salad)])
    } else if at(m, 9, 30) {
        plan(&[Sport])
    } else if at(m, 12, 30) {
        if s.dirt > pct_lvl(45) {
            plan(&[Wash, Eat(Pizza)])
        } else {
            plan(&[Eat(Pizza)])
        }
    } else if at(m, 16, 30) {
        plan(&[Sport])
    } else if at(m, 18, 30) {
        if s.fullness < pct_lvl(45) {
            plan(&[Eat(Apple), Eat(Salad)])
        } else {
            plan(&[Eat(Apple)])
        }
    } else if at(m, 17, 0) {
        plan(&[Wash])
    } else if at(m, 20, 30) {
        plan(&[Play])
    } else if at(m, 22, 30) {
        plan(&[Sleep])
    } else {
        Plan::new()
    };
    habits(p, s, true)
}

/// Sleeps late, lives on pizza, cake and cola. No weight habit.
pub fn lazy_treats(d: u32, m: u32, s: &GameState) -> Plan {
    use Act::*;
    use FoodKind::*;
    let p = if at(m, 10, 0) {
        plan(&[Wake, Eat(Pizza)])
    } else if at(m, 14, 0) {
        plan(&[Eat(Cake), Drink(DrinkKind::Cola)])
    } else if at(m, 16, 0) {
        if d % 2 == 0 {
            plan(&[Sport])
        } else {
            plan(&[Play])
        }
    } else if at(m, 19, 0) {
        plan(&[Eat(if s.fullness < pct_lvl(40) {
            Pizza
        } else {
            Frikandel
        })])
    } else if at(m, 21, 30) {
        if s.dirt > pct_lvl(55) {
            plan(&[Wash, Mini(1)])
        } else {
            plan(&[Mini(1)])
        }
    } else if at(m, 23, 59) {
        if s.fullness < pct_lvl(50) {
            plan(&[Eat(Apple)])
        } else {
            Plan::new()
        }
    } else if at(m, 1, 0) {
        plan(&[Sleep])
    } else {
        Plan::new()
    };
    habits(p, s, false)
}

/// Late nights, beer and friends. No weight habit.
pub fn party(d: u32, m: u32, s: &GameState) -> Plan {
    use Act::*;
    use FoodKind::*;
    let beer = Drink(DrinkKind::Beer);
    let water = Drink(DrinkKind::Water);
    let p = if at(m, 11, 0) {
        if s.dirt > pct_lvl(50) {
            plan(&[Wake, Wash, water, Eat(Frikandel)])
        } else {
            plan(&[Wake, water, Eat(Frikandel)])
        }
    } else if at(m, 14, 0) {
        if d % 2 == 0 {
            plan(&[Sport])
        } else {
            plan(&[Eat(Apple)])
        }
    } else if at(m, 16, 30) {
        plan(&[beer, Friend])
    } else if at(m, 19, 0) {
        plan(&[Eat(Pizza)])
    } else if at(m, 21, 0) {
        plan(&[beer, Friend])
    } else if at(m, 23, 30) {
        plan(&[Mini(2)])
    } else if at(m, 1, 30) {
        if s.fullness < pct_lvl(50) {
            plan(&[water, Eat(Apple)])
        } else {
            plan(&[water])
        }
    } else if at(m, 3, 0) {
        plan(&[Sleep])
    } else {
        Plan::new()
    };
    habits(p, s, false)
}

/// Balanced, but woken after five hours every night.
pub fn short_sleeper(_d: u32, m: u32, s: &GameState) -> Plan {
    if at(m, 4, 30) {
        return habits(plan(&[Act::Wake]), s, true);
    }
    let mut p = balanced_plan(m, s);
    p.retain(|a| *a != Act::Wake);
    habits(p, s, true)
}

fn awake_hours(m: u32) -> bool {
    m >= 8 * 60
}

/// Balanced without exercise and beer, plus `a` every `every` minutes awake.
fn spam_p(m: u32, every: u32, a: Act, s: &GameState) -> Plan {
    let mut v = balanced_plan(m, s);
    v.retain(|x| *x != Act::Sport && !matches!(x, Act::Drink(DrinkKind::Beer)));
    if awake_hours(m) && m % every == 0 {
        let _ = v.push(a);
    }
    v
}

/// Water every half hour on top of a sensible day.
pub fn spam_water(_d: u32, m: u32, s: &GameState) -> Plan {
    habits(spam_p(m, 30, Act::Drink(DrinkKind::Water), s), s, true)
}

/// A bath every half hour on top of a sensible day.
pub fn spam_wash(_d: u32, m: u32, s: &GameState) -> Plan {
    habits(spam_p(m, 30, Act::Wash, s), s, true)
}

/// One look a day at noon: a pizza and a play. No weight habit.
pub fn once_a_day(_d: u32, m: u32, s: &GameState) -> Plan {
    let p = if at(m, 12, 0) {
        plan(&[Act::Eat(FoodKind::Pizza), Act::Play])
    } else {
        Plan::new()
    };
    habits(p, s, false)
}

/// Never looks at the pet.
pub fn absent(_d: u32, _m: u32, _s: &GameState) -> Plan {
    Plan::new()
}
