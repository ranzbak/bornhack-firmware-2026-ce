//! Game balance simulator — runs all player profiles and outputs a summary
//! table matching the Python simulation format.
//!
//! Usage: `cargo run --bin simulate_game --features simulator`
//! Or:    `make simulate-game`

use bornhack_aegg::game::engine::sim::{
    self, ACT_ABOVE, Policy, SLEEP_ABOVE, Skip, WAKE_BELOW, night_owl_act, optimal_act,
};
use bornhack_aegg::game::engine::{FoodKind, GameState, Phase};

// ---------------------------------------------------------------------------
// Player policies (the shared logic lives in `game::engine::sim`)
// ---------------------------------------------------------------------------

/// Round `tick` up to the next multiple of `interval` (strictly later).
fn next_multiple(tick: u32, interval: u32) -> u32 {
    tick + interval - tick % interval
}

// ── Perfect: checks every tick ──────────────────────────────────────────

struct PerfectPolicy;
impl Policy for PerfectPolicy {
    fn name(&self) -> &'static str {
        "perfect"
    }
    fn act(&self, state: &mut GameState, _tick: u32) {
        optimal_act(state, Skip::NONE);
    }
}

// ── Perfect minus one action ────────────────────────────────────────────

struct PerfectSkip {
    label: &'static str,
    skip: Skip,
}
impl Policy for PerfectSkip {
    fn name(&self) -> &'static str {
        self.label
    }
    fn act(&self, state: &mut GameState, _tick: u32) {
        optimal_act(state, self.skip);
    }
}

// ── Interval-based attentive ────────────────────────────────────────────

struct AttentivePolicy {
    label: &'static str,
    interval: u32,
}

impl Policy for AttentivePolicy {
    fn name(&self) -> &'static str {
        self.label
    }
    fn act(&self, state: &mut GameState, _tick: u32) {
        optimal_act(state, Skip::NONE);
    }
    fn next_check(&self, tick: u32) -> u32 {
        next_multiple(tick, self.interval)
    }
}

// ── Night owl: never sleeps, checks every minute ────────────────────────

struct NightOwlPolicy;
impl Policy for NightOwlPolicy {
    fn name(&self) -> &'static str {
        "night_owl"
    }
    fn act(&self, state: &mut GameState, _tick: u32) {
        night_owl_act(state);
    }
    fn next_check(&self, tick: u32) -> u32 {
        next_multiple(tick, 6)
    }
}

// ── Absent: never interacts ─────────────────────────────────────────────

struct AbsentPolicy;
impl Policy for AbsentPolicy {
    fn name(&self) -> &'static str {
        "absent"
    }
    fn act(&self, _state: &mut GameState, _tick: u32) {}
    fn next_check(&self, _tick: u32) -> u32 {
        u32::MAX
    }
}

// ── Feed and sleep only / feed, sleep, heal ─────────────────────────────

/// Sleeps and wakes like the optimal player, but only treats hunger and,
/// optionally, sickness. Checks every 15 minutes.
struct LimitedCarePolicy {
    label: &'static str,
    heal: bool,
}
impl Policy for LimitedCarePolicy {
    fn name(&self) -> &'static str {
        self.label
    }
    fn act(&self, state: &mut GameState, _tick: u32) {
        if state.phase != Phase::Active {
            return;
        }
        if state.is_sleeping {
            if state.tired < WAKE_BELOW {
                state.wake();
            }
            return;
        }
        if state.tired > SLEEP_ABOVE {
            state.sleep();
            return;
        }
        if state.hunger > ACT_ABOVE {
            state.feed(FoodKind::Apple);
            return;
        }
        if self.heal && state.sick > ACT_ABOVE {
            state.heal();
        }
    }
    fn next_check(&self, tick: u32) -> u32 {
        next_multiple(tick, 90)
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let seed = 42u64;
    let max_days = 60u32;
    let ticks_per_hour = 360.0f64;

    let policies: Vec<Box<dyn Policy>> = vec![
        Box::new(PerfectPolicy),
        Box::new(PerfectSkip {
            label: "perfect_no_feed",
            skip: Skip { feed: true, ..Skip::NONE },
        }),
        Box::new(PerfectSkip {
            label: "perfect_no_heal",
            skip: Skip { heal: true, ..Skip::NONE },
        }),
        Box::new(PerfectSkip {
            label: "perfect_no_rest",
            skip: Skip { rest: true, ..Skip::NONE },
        }),
        Box::new(AttentivePolicy {
            label: "attentive_15min",
            interval: 90,
        }),
        Box::new(AttentivePolicy {
            label: "casual_30min",
            interval: 180,
        }),
        Box::new(AttentivePolicy {
            label: "busy_1hr",
            interval: 360,
        }),
        Box::new(NightOwlPolicy),
        Box::new(AbsentPolicy),
        Box::new(LimitedCarePolicy {
            label: "feed_and_sleep",
            heal: false,
        }),
        Box::new(LimitedCarePolicy {
            label: "feed_sleep_heal",
            heal: true,
        }),
    ];

    println!(
        "{:<22} {:>4} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>7}",
        "Profile",
        "Left",
        "Day",
        "Ticks",
        "Steps",
        "Hunger",
        "Tired",
        "Miser",
        "Sick",
        "Attn#",
        "Attn/h"
    );
    println!("{}", "─".repeat(102));

    for policy in &policies {
        let r = sim::run(policy.as_ref(), seed, max_days, true);
        let day_str = if r.left {
            format!("{:.1}", r.days)
        } else {
            format!("{:.1}+", r.days)
        };
        let attn_per_hour = if r.ticks > 0 {
            r.action_count as f64 / (r.ticks as f64 / ticks_per_hour)
        } else {
            0.0
        };

        println!(
            "{:<22} {:>4} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>7} {:>6} {:>7.2}",
            policy.name(),
            if r.left { "YES" } else { "no" },
            day_str,
            r.ticks,
            r.steps,
            r.hunger,
            r.tired,
            r.miserable,
            r.sick,
            r.action_count,
            attn_per_hour,
        );
    }
}
