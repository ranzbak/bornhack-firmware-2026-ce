//! Shared player-policy and simulation-runner code for the desktop balance
//! tools: `tests/game_engine.rs` and `src/bin/simulate_game.rs`.
//!
//! Host-only (`simulator` feature); nothing here is linked into firmware.

use super::{FoodKind, GameState, PetKind, Phase};

/// Wake a sleeping pet once `tired` drops below this (about 20 %).
pub const WAKE_BELOW: u16 = 13107;
/// Put an awake pet to sleep once `tired` exceeds this (about 70 %).
pub const SLEEP_ABOVE: u16 = 45874;
/// Reactive threshold: a stat above this (50 %) is treated.
pub const ACT_ABOVE: u16 = 32768;
/// Proactive threshold: feed when `hunger` exceeds this (25 %).
pub const PROACTIVE_FEED_ABOVE: u16 = 16384;

/// Ticks in a simulated day (1 tick = 10 s).
pub const TICKS_PER_DAY: u32 = 8640;

/// Which branches of [`optimal_act`] to leave out.
#[derive(Clone, Copy, Default)]
pub struct Skip {
    pub feed: bool,
    pub heal: bool,
    pub rest: bool,
    pub play: bool,
}

impl Skip {
    pub const NONE: Skip = Skip {
        feed: false,
        heal: false,
        rest: false,
        play: false,
    };
}

/// The optimal player: sleep/wake on `tired`, then feed > heal > play, then
/// proactive feeding. `skip` removes branches for the "perfect minus one
/// action" profiles (`rest` = never sleeps, wakes at once).
pub fn optimal_act(state: &mut GameState, skip: Skip) {
    if state.phase != Phase::Active {
        return;
    }

    if state.is_sleeping {
        if skip.rest || state.tired < WAKE_BELOW {
            state.wake();
        }
        return;
    }

    if !skip.rest && state.tired > SLEEP_ABOVE {
        state.sleep();
        return;
    }
    if !skip.feed && state.hunger > ACT_ABOVE {
        state.feed(FoodKind::Apple);
        return;
    }
    if !skip.heal && state.sick > ACT_ABOVE {
        state.heal();
        return;
    }
    if !skip.play && state.miserable > ACT_ABOVE {
        state.play();
        return;
    }
    if !skip.feed && state.hunger > PROACTIVE_FEED_ABOVE {
        state.feed(FoodKind::Apple);
    }
}

/// The night owl: never sleeps (wakes at once), otherwise feed > heal > play
/// at the reactive threshold, no proactive feeding.
pub fn night_owl_act(state: &mut GameState) {
    if state.phase != Phase::Active {
        return;
    }
    if state.is_sleeping {
        state.wake();
        return;
    }
    if state.hunger > ACT_ABOVE {
        state.feed(FoodKind::Apple);
        return;
    }
    if state.sick > ACT_ABOVE {
        state.heal();
        return;
    }
    if state.miserable > ACT_ABOVE {
        state.play();
    }
}

/// A simulated player.
pub trait Policy {
    /// Profile label for reports.
    fn name(&self) -> &'static str {
        ""
    }
    fn act(&self, state: &mut GameState, tick: u32);
    /// Next tick this policy wants to be called. The runner also wakes at
    /// the engine's own next boundary, whichever is sooner. Default: every
    /// tick; return `u32::MAX` to be woken by the engine only.
    fn next_check(&self, tick: u32) -> u32 {
        tick + 1
    }
}

/// Outcome of one [`run`].
pub struct RunResult {
    /// The pet left (phase `Gone`) before `max_days`.
    pub left: bool,
    /// Pet age in days at the end.
    pub days: f64,
    /// Pet age in ticks at the end.
    pub ticks: u32,
    /// Number of engine/policy wake-ups.
    pub steps: u32,
    pub hunger: u16,
    pub tired: u16,
    pub miserable: u16,
    pub sick: u16,
    /// Number of times the policy started a new action.
    pub action_count: u32,
}

/// Run `policy` against a fresh egg for up to `max_days` days.
///
/// Boundary-based scheduling: each step jumps to the sooner of the engine's
/// `next_wake_tick` and the policy's `next_check`, always advancing at least
/// one tick. `money` sets `GameState::money_enabled`; the policies never earn
/// HAX, so with the gate on they eventually cannot afford feed/heal/play.
pub fn run(policy: &dyn Policy, seed: u64, max_days: u32, money: bool) -> RunResult {
    let max_ticks = max_days * TICKS_PER_DAY;

    let mut state = GameState::new_egg(seed, PetKind::DEFAULT);
    state.money_enabled = money;
    let mut tick: u32 = 0;
    let mut action_count: u32 = 0;
    let mut steps: u32 = 0;

    while tick < max_ticks && state.phase != Phase::Gone {
        state.update(tick);
        let before = state.active_action;
        policy.act(&mut state, tick);
        if state.active_action != before && state.active_action.is_some() {
            action_count += 1;
        }

        let next_engine = state.next_wake_tick();
        let next_policy = policy.next_check(tick);
        tick = next_engine.min(next_policy).max(tick + 1).min(max_ticks);
        steps += 1;
    }

    RunResult {
        left: state.phase == Phase::Gone,
        days: state.age_ticks as f64 / TICKS_PER_DAY as f64,
        ticks: state.age_ticks,
        steps,
        hunger: state.hunger,
        tired: state.tired,
        miserable: state.miserable,
        sick: state.sick,
        action_count,
    }
}
