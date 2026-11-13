//! Player actions as doses.
//!
//! An action's effect lands in one piece when its animation ends
//! ([`apply`]); no time passes inside a dose, the needs only age in
//! `update`'s segments. Refusals are decided by [`refusal`], which the
//! starters call before waking or charging and `apply` calls again, so the
//! rule exists once. Every dose number is a `Thresholds` field.

use super::needs::{self, MAX, P};
use super::thresholds as t;
use super::{Action, DrinkKind, FoodKind, GameState};

/// Why an action does nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum Refusal {
    NotThirsty,
    AlreadyClean,
    Injured,
    Asleep,
    NoNeed,
}

/// Water is refused above this hydration and never takes it past `WATER_CAP_PCT`
/// (fixed safety edges of the hydration need, see `needs::build_edges`).
const WATER_REFUSE_PCT: i32 = 85;
const WATER_CAP_PCT: i32 = 90;

/// A level shifted by `delta`, clamped to the level range.
fn bump(level: &mut u16, delta: i32) {
    *level = (*level as i32 + delta).clamp(0, MAX) as u16;
}

/// Percent points as a level delta.
fn lvl(pct: u8) -> i32 {
    pct as i32 * P
}

/// Whether `a` (with the chosen `drink`, for `Action::Drink`) would be refused.
pub fn refusal(s: &GameState, a: Action, drink: Option<DrinkKind>) -> Option<Refusal> {
    match (a, drink) {
        (Action::Drink, Some(DrinkKind::Water)) if s.hydration as i32 > WATER_REFUSE_PCT * P => {
            Some(Refusal::NotThirsty)
        }
        _ => None,
    }
}

/// Apply the dose of the action that just ended.
pub fn apply(s: &mut GameState, a: Action) -> Result<(), Refusal> {
    if let Some(r) = refusal(s, a, s.active_drink) {
        return Err(r);
    }
    match a {
        Action::Feed => {
            if let Some(food) = s.active_food {
                eat(s, food);
            }
        }
        Action::Drink => {
            if let Some(drink) = s.active_drink {
                sip(s, drink);
            }
        }
        // Task 9b: Play, Exercise, Wash, Relax, Heal, Medicate, Ozempic,
        // Rehab, Sleep, Wake, OnlyPets and the mini-game / friend doses.
        _ => {}
    }
    Ok(())
}

/// What one food does, from the thresholds.
struct Meal {
    full: u8,
    dirt: u8,
    weight_pm: u16,
    mood: u8,
    healthy: bool,
}

fn meal(food: FoodKind) -> Meal {
    let (full, dirt, weight_pm, mood, healthy) = match food {
        FoodKind::Salad => (
            t::DOSE_SALAD_FULL_PCT(),
            t::DOSE_SALAD_DIRT_PCT(),
            t::DOSE_SALAD_WEIGHT_PM(),
            0,
            true,
        ),
        FoodKind::Apple => (
            t::DOSE_APPLE_FULL_PCT(),
            t::DOSE_APPLE_DIRT_PCT(),
            t::DOSE_APPLE_WEIGHT_PM(),
            0,
            true,
        ),
        FoodKind::Frikandel => (
            t::DOSE_FRIKANDEL_FULL_PCT(),
            t::DOSE_FRIKANDEL_DIRT_PCT(),
            t::DOSE_FRIKANDEL_WEIGHT_PM(),
            t::DOSE_FRIKANDEL_MOOD_PCT(),
            false,
        ),
        FoodKind::Pizza => (
            t::DOSE_PIZZA_FULL_PCT(),
            t::DOSE_PIZZA_DIRT_PCT(),
            t::DOSE_PIZZA_WEIGHT_PM(),
            t::DOSE_PIZZA_MOOD_PCT(),
            false,
        ),
        FoodKind::Cake => (
            t::DOSE_CAKE_FULL_PCT(),
            t::DOSE_CAKE_DIRT_PCT(),
            t::DOSE_CAKE_WEIGHT_PM(),
            t::DOSE_CAKE_MOOD_PCT(),
            false,
        ),
    };
    Meal {
        full,
        dirt,
        weight_pm,
        mood,
        healthy,
    }
}

fn eat(s: &mut GameState, food: FoodKind) {
    let m = meal(food);
    // Eating past the band, counted up to the cap even though the stored
    // level tops out at 100 %, is a food coma and goes to the waist.
    let after = s.fullness as i32 + lvl(m.full);
    let excess = (after.min(lvl(t::DOSE_FULL_CAP_PCT())) - needs::full_hi()).max(0);
    bump(&mut s.fullness, lvl(m.full));
    bump(&mut s.dirt, lvl(m.dirt));
    bump(
        &mut s.weight,
        m.weight_pm as i32 * P / 10 + excess / t::DOSE_EXCESS_WEIGHT_DIV() as i32,
    );
    bump(
        &mut s.energy,
        -(excess / t::DOSE_EXCESS_ENERGY_DIV() as i32),
    );
    if m.healthy {
        bump(&mut s.hydration, lvl(t::DOSE_HEALTHY_WATER_PCT()));
    }
    if food == FoodKind::Salad {
        bump(&mut s.fitness, lvl(t::DOSE_SALAD_FIT_PCT()));
    }
    if m.mood > 0 {
        // A treat: full mood only when the last one was over three hours ago,
        // none at all on Ozempic; either way the window restarts.
        if s.ozempic_min == 0 {
            let pct = if s.junk_min == 0 {
                m.mood
            } else {
                m.mood / t::DOSE_JUNK_REPEAT_DIV()
            };
            bump(&mut s.mood, lvl(pct));
        }
        s.junk_min = t::DOSE_JUNK_MIN();
    }
}

/// What one non-water drink does, from the thresholds.
struct Pour {
    water: u8,
    belly: u8,
    booze: u8,
    mood: u8,
    energy: u8,
    fun: u8,
    weight: u8,
}

fn pour(drink: DrinkKind) -> Pour {
    let alcohol = |water, belly, booze, mood, weight| Pour {
        water,
        belly,
        booze,
        mood,
        energy: t::DOSE_ALCOHOL_ENERGY_PCT(),
        fun: t::DOSE_ALCOHOL_FUN_PCT(),
        weight,
    };
    match drink {
        DrinkKind::Cola => Pour {
            water: t::DOSE_COLA_WATER_PCT(),
            belly: t::DOSE_COLA_FULL_PCT(),
            booze: 0,
            mood: t::DOSE_COLA_MOOD_PCT(),
            energy: t::DOSE_COLA_ENERGY_PCT(),
            fun: 0,
            weight: t::DOSE_COLA_WEIGHT_PCT(),
        },
        DrinkKind::Beer => alcohol(
            t::DOSE_BEER_WATER_PCT(),
            t::DOSE_BEER_BELLY_PCT(),
            t::DOSE_BEER_BOOZE_PCT(),
            t::DOSE_BEER_MOOD_PCT(),
            t::DOSE_BEER_WEIGHT_PCT(),
        ),
        DrinkKind::Wine => alcohol(
            t::DOSE_WINE_WATER_PCT(),
            t::DOSE_WINE_BELLY_PCT(),
            t::DOSE_WINE_BOOZE_PCT(),
            t::DOSE_WINE_MOOD_PCT(),
            0,
        ),
        DrinkKind::Whiskey => alcohol(
            t::DOSE_WHISKEY_WATER_PCT(),
            t::DOSE_WHISKEY_BELLY_PCT(),
            t::DOSE_WHISKEY_BOOZE_PCT(),
            t::DOSE_WHISKEY_MOOD_PCT(),
            0,
        ),
        DrinkKind::Water => unreachable!("water has its own dose"),
    }
}

fn sip(s: &mut GameState, drink: DrinkKind) {
    if drink == DrinkKind::Water {
        // A pet only drinks what it can hold: never past WATER_CAP_PCT.
        let room = (WATER_CAP_PCT * P - s.hydration as i32).max(0) / P;
        let taken = room.min(t::DOSE_WATER_MAX_PCT() as i32);
        bump(&mut s.hydration, taken * P);
        if taken >= t::DOSE_WATER_BELLY_MIN_PCT() as i32 {
            bump(&mut s.fullness, lvl(t::DOSE_WATER_BELLY_PCT()));
        }
        bump(&mut s.booze, -lvl(t::DOSE_WATER_BOOZE_PCT()));
        s.hangover_min = s.hangover_min.saturating_sub(t::DOSE_WATER_HANGOVER_MIN());
        s.groggy_min = s.groggy_min.saturating_sub(t::DOSE_WATER_GROGGY_MIN());
        return;
    }

    let p = pour(drink);
    let alcoholic = p.booze > 0;
    if alcoholic && s.rehab_min > 0 {
        // Relapse.
        s.rehab_min = 0;
        bump(&mut s.mood, -lvl(t::DOSE_RELAPSE_MOOD_PCT()));
    }
    // The state the pet was in when the glass came: tired hits harder, a
    // drunk pet gets less out of it, a dependent one half of that.
    let tired = (s.energy as i32) < needs::energy_lo();
    let mut dim = (100 - needs::pct(s.booze as i32)).max(0);
    if s.dependent {
        dim /= t::DOSE_DEPENDENT_DIV() as i32;
    }
    let amp = if tired {
        t::DOSE_TIRED_BOOZE_PCT() as i32
    } else {
        100
    };

    bump(&mut s.hydration, lvl(p.water));
    bump(&mut s.fullness, lvl(p.belly));
    bump(&mut s.weight, lvl(p.weight));
    bump(&mut s.fun, lvl(p.fun));
    if alcoholic {
        bump(&mut s.booze, lvl(p.booze) * amp / 100);
        bump(&mut s.mood, lvl(p.mood) * dim / 100);
        bump(&mut s.energy, lvl(p.energy) * dim / 100);
    } else {
        bump(&mut s.mood, lvl(p.mood));
        bump(&mut s.energy, lvl(p.energy));
    }
}
