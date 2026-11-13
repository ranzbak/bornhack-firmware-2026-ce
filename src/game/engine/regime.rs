//! Per-minute need rates of the pet's current regime (awake or asleep).
//!
//! Rates are signed levels per engine minute on the `u16` scale. Every
//! magnitude and multiplier is a [`thresholds`] field; the sign lives here.
//! Multipliers are applied in the order listed, each as an integer
//! `x * pct / 100` that truncates toward zero.

use super::GameState;
use super::needs::{self, MAX, NEEDS, Need, P};
use super::thresholds as t;

#[inline]
fn lvl(pct: u8) -> i32 {
    pct as i32 * P
}

#[inline]
fn pct_of(x: i32, pct: u16) -> i32 {
    x * pct as i32 / 100
}

/// Thirst multiplier (percent) from the booze level: it dries the pet out.
pub fn thirst_pct(s: &GameState) -> i32 {
    let b = s.booze as i32;
    (if b < lvl(t::THIRST_BOOZE_LOW_PCT()) {
        t::THIRST_MULT_BASE_PCT()
    } else if b < lvl(t::THIRST_BOOZE_MID_PCT()) {
        t::THIRST_MULT_LOW_PCT()
    } else if b < lvl(t::THIRST_BOOZE_HIGH_PCT()) {
        t::THIRST_MULT_MID_PCT()
    } else {
        t::THIRST_MULT_HIGH_PCT()
    }) as i32
}

/// Energy ceiling while asleep: lower when the pet is deprived.
pub fn energy_cap(s: &GameState) -> i32 {
    if s.deprived { lvl(t::ENERGY_CAP_DEPRIVED_PCT()) } else { MAX }
}

/// Sleep recovery quality in per-mille: poor when the pet is unsettled.
pub fn sleep_quality_pm(s: &GameState) -> i32 {
    let (b, f, h, fun) = (s.booze as i32, s.fullness as i32, s.hydration as i32, s.fun as i32);
    let unsettled = b > lvl(t::UNSETTLED_BOOZE_PCT())
        || f > lvl(t::UNSETTLED_FULL_HI_PCT())
        || f < lvl(t::UNSETTLED_FULL_LO_PCT())
        || h < lvl(t::UNSETTLED_WATER_LO_PCT())
        || h > lvl(t::UNSETTLED_WATER_HI_PCT())
        || fun > needs::fun_hi()
        || s.rehab_min > 0;
    (if unsettled { t::SLEEP_QUALITY_POOR_PM() } else { t::SLEEP_QUALITY_GOOD_PM() }) as i32
}

/// Awake energy drain multiplier in per-mille (1000 = base rate).
fn energy_drain_pm(s: &GameState) -> i32 {
    let (fit, weight) = (s.fitness as i32, s.weight as i32);
    let mut m = 1000;
    if fit < needs::fit_lo() {
        m = pct_of(m, t::ENERGY_MULT_UNFIT_PCT());
    }
    if fit >= lvl(t::ENERGY_FIT_GOOD_PCT()) {
        m = pct_of(m, t::ENERGY_MULT_FIT_PCT());
    }
    if s.booze as i32 > needs::booze_hi() {
        m = pct_of(m, t::ENERGY_MULT_DRUNK_PCT());
    }
    if s.fun as i32 > needs::fun_hi() {
        m = pct_of(m, t::ENERGY_MULT_WIRED_PCT());
    }
    if s.hangover_min > 0 {
        m = pct_of(m, t::ENERGY_MULT_HANGOVER_PCT());
    }
    if weight > needs::weight_hi() || weight < needs::weight_lo() {
        m = pct_of(m, t::ENERGY_MULT_WEIGHT_PCT());
    }
    m
}

/// Per-minute rate of each need, indexed by `Need as usize`.
pub fn rates(s: &GameState) -> [i32; NEEDS] {
    let thirst = thirst_pct(s);
    let (full, water, energy, fun);
    if s.is_sleeping {
        full = -(t::FULL_DRAIN_ASLEEP_PM() as i32);
        water = -(t::WATER_DRAIN_ASLEEP_PM() as i32) * thirst / 100;
        energy = if (s.energy as i32) < energy_cap(s) {
            t::ENERGY_GAIN_ASLEEP_PM() as i32 * sleep_quality_pm(s) / 1000
        } else {
            0
        };
        fun = if s.fun as i32 > lvl(t::SLEEP_FUN_FLOOR_PCT()) { -(t::FUN_DRAIN_PM() as i32) } else { 0 };
    } else {
        full = -(t::FULL_DRAIN_AWAKE_PM() as i32);
        water = -(t::WATER_DRAIN_AWAKE_PM() as i32) * thirst / 100;
        energy = -(t::ENERGY_DRAIN_AWAKE_PM() as i32) * energy_drain_pm(s) / 1000;
        fun = -(t::FUN_DRAIN_PM() as i32);
    }
    let (fit, weight, fullness) = (s.fitness as i32, s.weight as i32, s.fullness as i32);
    let heavy = weight > needs::weight_hi();

    let mut booze = t::BOOZE_CLEAR_PM() as i32;
    if (s.hydration as i32) < needs::water_lo() {
        booze = pct_of(booze, t::BOOZE_CLEAR_SLOW_PCT());
    }
    if s.dependent {
        booze = pct_of(booze, t::BOOZE_CLEAR_SLOW_PCT());
    }

    let mut fit_decay = if fit > lvl(t::FIT_DECAY_HIGH_ABOVE_PCT()) {
        t::FIT_DECAY_HIGH_PM()
    } else if fit > lvl(t::FIT_DECAY_MID_ABOVE_PCT()) {
        t::FIT_DECAY_MID_PM()
    } else {
        t::FIT_DECAY_LOW_PM()
    } as i32;
    if heavy {
        fit_decay = pct_of(fit_decay, t::FIT_DECAY_HEAVY_PCT());
    }

    // Metabolism pulls weight back toward its band; an empty or stuffed belly shifts it.
    let metabolism = if heavy {
        -(t::WEIGHT_PULL_HEAVY_PM() as i32)
    } else if weight < needs::weight_lo() {
        0
    } else {
        -(t::WEIGHT_PULL_PM() as i32)
    };
    let belly = if fullness < needs::full_lo() {
        -(t::WEIGHT_BELLY_PM() as i32)
    } else if fullness > needs::full_hi() {
        t::WEIGHT_BELLY_PM() as i32
    } else {
        0
    };

    let mut r = [0; NEEDS];
    r[Need::Full as usize] = full;
    r[Need::Water as usize] = water;
    r[Need::Energy as usize] = energy;
    r[Need::Fun as usize] = fun;
    r[Need::Dirt as usize] = t::DIRT_GAIN_PM() as i32;
    r[Need::Booze as usize] = -booze;
    r[Need::Strain as usize] = -(t::STRAIN_CLEAR_PM() as i32);
    r[Need::Fit as usize] = -fit_decay;
    r[Need::Weight as usize] = metabolism + belly;
    r
}
