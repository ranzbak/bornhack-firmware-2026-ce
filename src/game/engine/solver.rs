//! Segment solver: how many minutes `advance` may take in one step.
//!
//! A segment ends one minute before anything `advance` or `regime` branches
//! on can change: a need level reaching a threshold, a minute timer
//! expiring, or a slow counter reaching its onset line. Inside a segment the
//! rates, penalties and mood/health terms are therefore constant, so the
//! closed-form `advance` is exact (R17, R18). Mood and health crossing their
//! own lines are not handled here.

use super::advance::{LEAVING_SCALE, SLOW_SCALE};
use super::needs::{self, MAX, NEEDS, Need, P};
use super::regime::{energy_cap, rates};
use super::thresholds as t;
use super::{GameState, Phase};

#[inline]
fn lvl(pct: u8) -> i32 {
    pct as i32 * P
}

/// `ceil(a / b)` for an amount that may already be spent (then 0).
#[inline]
fn ceil_div(a: i64, b: i64) -> i64 {
    if a <= 0 { 0 } else { (a + b - 1) / b }
}

/// Levels a need's rate or a penalty branches on that the edge table does
/// not guarantee: its fixed rows hold literals while these are tunable, and
/// the table lacks the thirst steps (booze 40 %) outright.
const MAX_EXTRA: usize = 8;

fn extra_levels(s: &GameState, need: Need) -> ([i32; MAX_EXTRA], usize) {
    let mut v = [0; MAX_EXTRA];
    let n = match need {
        Need::Full => {
            v[..4].copy_from_slice(&[
                lvl(t::UNSETTLED_FULL_LO_PCT()),
                lvl(t::UNSETTLED_FULL_HI_PCT()),
                lvl(t::DMG_NEED_LOW_PCT()),
                lvl(t::DMG_NEED_HIGH_PCT()),
            ]);
            4
        }
        Need::Water => {
            v[..4].copy_from_slice(&[
                lvl(t::UNSETTLED_WATER_LO_PCT()),
                lvl(t::UNSETTLED_WATER_HI_PCT()),
                lvl(t::DMG_NEED_LOW_PCT()),
                lvl(t::DMG_NEED_HIGH_PCT()),
            ]);
            4
        }
        Need::Energy => {
            v[..3].copy_from_slice(&[
                lvl(t::DMG_ENERGY_PCT()),
                lvl(t::OVERSLEEP_RESET_PCT()),
                energy_cap(s),
            ]);
            3
        }
        Need::Fun => {
            v[0] = lvl(t::SLEEP_FUN_FLOOR_PCT());
            1
        }
        Need::Dirt => {
            v[0] = lvl(t::DMG_DIRT_PCT());
            1
        }
        Need::Booze => {
            v[..6].copy_from_slice(&[
                lvl(t::THIRST_BOOZE_LOW_PCT()),
                lvl(t::THIRST_BOOZE_MID_PCT()),
                lvl(t::THIRST_BOOZE_HIGH_PCT()),
                lvl(t::BOOZE_CLEAR_PCT()),
                lvl(t::MOOD_CRAVING_BOOZE_PCT()),
                lvl(t::UNSETTLED_BOOZE_PCT()),
            ]);
            6
        }
        Need::Strain => 0,
        Need::Fit => {
            v[..6].copy_from_slice(&[
                lvl(t::ENERGY_FIT_GOOD_PCT()),
                lvl(t::FIT_DECAY_HIGH_ABOVE_PCT()),
                lvl(t::FIT_DECAY_MID_ABOVE_PCT()),
                lvl(t::REVERSAL_FIT_PCT()),
                lvl(t::CEILING_FIT_STEP1_PCT()),
                lvl(t::CEILING_FIT_STEP2_PCT()),
            ]);
            v[6] = lvl(t::CEILING_FIT_STEP3_PCT());
            7
        }
        Need::Weight => {
            v[..4].copy_from_slice(&[
                lvl(t::DIABETES_WEIGHT_HI_PCT()),
                lvl(t::DIABETES_WEIGHT_LO_PCT()),
                lvl(t::DMG_WEIGHT_HIGH_PCT()),
                lvl(t::DMG_WEIGHT_LOW_PCT()),
            ]);
            4
        }
    };
    (v, n)
}

const NEED_ORDER: [Need; NEEDS] = [
    Need::Full,
    Need::Water,
    Need::Energy,
    Need::Fun,
    Need::Dirt,
    Need::Booze,
    Need::Strain,
    Need::Fit,
    Need::Weight,
];

/// Minutes until `level`, moving `rate` per minute, is one unit short of
/// `target`; `None` when `target` is behind it. A level already on the target
/// and moving off it flips a `>` / `<` test next minute, so that gives 0.
#[inline]
fn minutes_to(level: i32, rate: i32, target: i32) -> Option<i32> {
    let dist = target - level;
    if rate > 0 && dist >= 0 || rate < 0 && dist <= 0 {
        Some((dist.abs() - 1).max(0) / rate.abs())
    } else {
        None
    }
}

/// Shortest time any moving, unpinned need takes to reach a threshold.
fn level_bound(s: &GameState) -> u32 {
    let r = rates(s);
    let levels = [
        s.fullness,
        s.hydration,
        s.energy,
        s.fun,
        s.dirt,
        s.booze,
        s.strain,
        s.fitness,
        s.weight,
    ];
    let table = needs::edges();
    let mut best = u32::MAX;
    for i in 0..NEEDS {
        let (level, rate) = (levels[i] as i32, r[i]);
        if rate == 0 || rate < 0 && level == 0 || rate > 0 && level == MAX {
            continue;
        }
        let need = NEED_ORDER[i];
        let (extra, extras) = extra_levels(s, need);
        let tabled = table.edges[i][..table.len[i] as usize]
            .iter()
            .map(|&p| lvl(p));
        let fixed = [0, MAX];
        for target in tabled.chain(extra[..extras].iter().copied()).chain(fixed) {
            if let Some(n) = minutes_to(level, rate, target) {
                best = best.min(n as u32);
            }
        }
    }
    best
}

/// Minutes until a timer that is running expires (it is read as 0 after).
fn timer_bound(s: &GameState) -> u32 {
    let mut best = i64::MAX;
    let mut running = |minutes: i64| {
        if minutes > 0 {
            best = best.min(minutes);
        }
    };
    let asleep = s.is_sleeping;
    let hangover_mult = if asleep {
        t::HANGOVER_ASLEEP_MULT() as i64
    } else {
        1
    };
    running(ceil_div(s.hangover_min as i64, hangover_mult));
    for m in [
        s.rested_min,
        s.injured_min,
        s.groggy_min,
        s.med_cd_min,
        s.recovering_min,
        s.junk_min,
        s.insulin_min,
        s.ozempic_min,
        s.ozempic_cd_min,
        s.rehab_min,
    ] {
        running(m as i64);
    }

    if asleep {
        running(t::SELF_WAKE_MIN() as i64 - s.sleep_min as i64);
        if s.energy as i32 >= energy_cap(s) {
            running(t::OVERSLEEP_WAKE_MIN() as i64 - s.oversleep_min as i64);
            running(t::GROGGY_LINE_MIN() as i64 + 1 - s.oversleep_min as i64);
        }
    }

    let scaled_left = |line_min: u16, scale: u32, counter: u32| {
        ceil_div(
            line_min as i64 * scale as i64 - counter as i64,
            scale as i64,
        )
    };
    if s.booze as i32 > needs::booze_hi() && !s.dependent {
        let line = if s.recovering_min > 0 {
            t::DEPENDENCE_RELAPSE_MIN()
        } else {
            t::DEPENDENCE_ONSET_MIN()
        };
        running(scaled_left(line, SLOW_SCALE, s.dependence_min));
    }
    if s.weight as i32 > lvl(t::DIABETES_WEIGHT_HI_PCT()) && !s.diabetic {
        running(scaled_left(
            t::DIABETES_ONSET_MIN(),
            SLOW_SCALE,
            s.diabetes_min,
        ));
    }
    if s.diabetic
        && (s.weight as i32) < lvl(t::DIABETES_WEIGHT_LO_PCT())
        && s.fitness as i32 > lvl(t::REVERSAL_FIT_PCT())
    {
        running(t::DIABETES_REVERSAL_MIN() as i64 - s.reversal_min as i64);
    }

    if s.health == 0 {
        running(scaled_left(t::LEAVING_MIN(), LEAVING_SCALE, s.leaving_min));
    } else {
        // Counting down; reaching 0 returns a Leaving pet to Active.
        running(s.leaving_min as i64);
    }
    best.min(u32::MAX as i64) as u32
}

/// Longest segment (1..=`max` minutes) over which no threshold is crossed and
/// no timer expires. Pure: reads the state, changes nothing.
pub fn seg_len(s: &GameState, max: u32) -> u32 {
    if s.phase == Phase::Gone {
        return max.max(1);
    }
    max.min(level_bound(s)).min(timer_bound(s)).max(1)
}
