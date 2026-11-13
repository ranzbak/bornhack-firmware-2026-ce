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
/// Wash is refused below this dirt level (fixed safety edge of the dirt need).
const WASH_REFUSE_PCT: i32 = 25;

/// A level shifted by `delta`, clamped to the level range.
fn bump(level: &mut u16, delta: i32) {
    *level = (*level as i32 + delta).clamp(0, MAX) as u16;
}

/// Percent points as a level delta.
fn lvl(pct: u8) -> i32 {
    pct as i32 * P
}

/// Whether `a` (with the chosen `drink`, for `Action::Drink`) would be refused.
/// Medical preconditions and cooldowns that are not met are `NoNeed`; a pet
/// too tired to work is `NoNeed` too, an injured one `Injured`.
pub fn refusal(s: &GameState, a: Action, drink: Option<DrinkKind>) -> Option<Refusal> {
    let injured = s.injured_min > 0;
    match (a, drink) {
        (Action::Drink, Some(DrinkKind::Water)) if s.hydration as i32 > WATER_REFUSE_PCT * P => {
            Some(Refusal::NotThirsty)
        }
        (Action::Play | Action::Exercise, _) if injured => Some(Refusal::Injured),
        (Action::Wash, _) if (s.dirt as i32) < WASH_REFUSE_PCT * P => Some(Refusal::AlreadyClean),
        (Action::OnlyPets, _) if s.is_sleeping => Some(Refusal::Asleep),
        (Action::OnlyPets, _) if injured => Some(Refusal::Injured),
        (Action::OnlyPets, _) if (s.energy as i32) < needs::energy_lo() => Some(Refusal::NoNeed),
        (Action::Heal, _) if s.med_cd_min > 0 => Some(Refusal::NoNeed),
        (Action::Medicate, _) if !s.diabetic || s.insulin_min > t::DOSE_INSULIN_WINDOW_MIN() => {
            Some(Refusal::NoNeed)
        }
        (Action::Ozempic, _) if s.ozempic_cd_min > 0 => Some(Refusal::NoNeed),
        (Action::Rehab, _) if !s.dependent || s.rehab_min > 0 => Some(Refusal::NoNeed),
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
        Action::Play => play(s),
        Action::Exercise => exercise(s),
        Action::Wash => bump(&mut s.dirt, -lvl(t::DOSE_WASH_DIRT_PCT())),
        Action::Relax => relax(s),
        Action::Heal => heal(s),
        Action::Medicate => s.insulin_min = t::DOSE_INSULIN_MIN(),
        Action::Ozempic => ozempic(s),
        Action::Rehab => s.rehab_min = t::DOSE_REHAB_MIN(),
        Action::OnlyPets => work(s),
    }
    Ok(())
}

/// Tired pets get twice the fun out of play (and mini-games).
fn fun_for(s: &GameState, pct: i32) -> i32 {
    let mult = if (s.energy as i32) < needs::energy_lo() {
        t::DOSE_TIRED_FUN_MULT() as i32
    } else {
        1
    };
    pct * P * mult
}

fn play(s: &mut GameState) {
    let fun = fun_for(s, t::DOSE_PLAY_FUN_PCT() as i32);
    bump(&mut s.fun, fun);
    bump(&mut s.energy, -lvl(t::DOSE_PLAY_ENERGY_PCT()));
    bump(&mut s.fullness, -lvl(t::DOSE_PLAY_FULL_PCT()));
    bump(&mut s.hydration, -lvl(t::DOSE_PLAY_WATER_PCT()));
    bump(&mut s.fitness, lvl(t::DOSE_PLAY_FIT_PCT()));
    bump(&mut s.dirt, lvl(t::DOSE_PLAY_DIRT_PCT()));
    s.groggy_min = s.groggy_min.saturating_sub(t::DOSE_PLAY_GROGGY_MIN());
}

/// A finished mini-game of grade `grade` (0..=2).
pub fn minigame(s: &mut GameState, grade: u8) {
    let pct =
        t::DOSE_MINI_FUN_BASE_PCT() as i32 + t::DOSE_MINI_FUN_GRADE_PCT() as i32 * grade as i32;
    let fun = fun_for(s, pct);
    bump(&mut s.fun, fun);
    bump(&mut s.energy, -lvl(t::DOSE_MINI_ENERGY_PCT()));
}

/// Meeting a mesh friend: a new one lifts the fun a lot, a known one a little.
pub fn friend(s: &mut GameState, big: bool) {
    let pct = if big {
        t::DOSE_FRIEND_BIG_FUN_PCT()
    } else {
        t::DOSE_FRIEND_SMALL_FUN_PCT()
    };
    bump(&mut s.fun, lvl(pct));
}

fn exercise(s: &mut GameState) {
    // Every multiplier looks at the pet as it was before the session.
    let mut strain = lvl(t::DOSE_EXERCISE_STRAIN_PCT());
    if (s.fullness as i32) < needs::full_lo()
        || (s.energy as i32) < needs::energy_lo()
        || (s.hydration as i32) < needs::water_lo()
    {
        strain = strain * t::DOSE_STRAIN_NEED_PCT() as i32 / 100;
    }
    if s.rested_min > 0 {
        strain = strain * t::DOSE_STRAIN_RESTED_PCT() as i32 / 100;
    }
    if s.fitness as i32 >= lvl(t::DOSE_STRAIN_FIT_MIN_PCT()) {
        strain = strain * t::DOSE_STRAIN_FIT_PCT() as i32 / 100;
    }
    if s.weight as i32 > needs::weight_hi() {
        strain = strain * t::DOSE_STRAIN_HEAVY_PCT() as i32 / 100;
    }
    if s.deprived {
        strain = strain * t::DOSE_STRAIN_DEPRIVED_PCT() as i32 / 100;
    }
    bump(&mut s.strain, strain);
    bump(&mut s.energy, -lvl(t::DOSE_EXERCISE_ENERGY_PCT()));
    bump(&mut s.fullness, -lvl(t::DOSE_EXERCISE_FULL_PCT()));
    bump(&mut s.hydration, -lvl(t::DOSE_EXERCISE_WATER_PCT()));
    bump(&mut s.fun, lvl(t::DOSE_EXERCISE_FUN_PCT()));
    bump(&mut s.booze, -lvl(t::DOSE_EXERCISE_BOOZE_PCT()));
    bump(&mut s.dirt, lvl(t::DOSE_EXERCISE_DIRT_PCT()));
    bump(&mut s.weight, -lvl(t::DOSE_EXERCISE_WEIGHT_PCT()));
    s.hangover_min = s
        .hangover_min
        .saturating_sub(t::DOSE_EXERCISE_HANGOVER_MIN());
    s.groggy_min = 0;
    if s.strain as i32 > needs::strain_inj() {
        s.injured_min = t::DOSE_INJURY_MIN();
        bump(&mut s.health, -lvl(t::DOSE_INJURY_HEALTH_PCT()));
    } else {
        bump(&mut s.fitness, lvl(t::DOSE_EXERCISE_FIT_PCT()));
    }
}

fn relax(s: &mut GameState) {
    bump(&mut s.fun, -lvl(t::DOSE_RELAX_FUN_PCT()));
    bump(&mut s.strain, -lvl(t::DOSE_RELAX_STRAIN_PCT()));
    bump(&mut s.energy, lvl(t::DOSE_RELAX_ENERGY_PCT()));
    bump(&mut s.fitness, -lvl(t::DOSE_RELAX_FIT_PCT()));
}

/// Medicine: halves an injury (and numbs: booze), does nothing but numb a
/// healthy pet, mends a sick one.
fn heal(s: &mut GameState) {
    s.med_cd_min = t::DOSE_MED_CD_MIN();
    if s.injured_min > 0 {
        s.injured_min = s.injured_min.saturating_sub(t::DOSE_HEAL_INJURY_MIN());
        bump(&mut s.booze, lvl(t::DOSE_MED_BOOZE_PCT()));
    } else if s.health as i32 > lvl(t::DOSE_HEAL_HEALTHY_PCT()) {
        bump(&mut s.booze, lvl(t::DOSE_MED_BOOZE_PCT()));
    } else {
        bump(&mut s.health, lvl(t::DOSE_HEAL_HEALTH_PCT()));
    }
}

fn ozempic(s: &mut GameState) {
    bump(&mut s.weight, -lvl(t::DOSE_OZEMPIC_WEIGHT_PCT()));
    bump(&mut s.fitness, -lvl(t::DOSE_OZEMPIC_FIT_PCT()));
    s.ozempic_min = t::DOSE_OZEMPIC_MIN();
    s.ozempic_cd_min = t::DOSE_OZEMPIC_CD_MIN();
}

/// "Only pets": the pet works for HAX (plan 6 replaces the flat pay).
fn work(s: &mut GameState) {
    bump(&mut s.energy, -lvl(t::DOSE_WORK_ENERGY_PCT()));
    bump(&mut s.fun, lvl(t::DOSE_WORK_FUN_PCT()));
    bump(&mut s.mood, -lvl(t::DOSE_WORK_MOOD_PCT()));
    if s.money_enabled {
        s.add_money(super::ONLYPETS_HOBBY_REWARD);
    }
}

/// Lie down. `night` is decided by how tired the pet is now: a worn-out pet
/// starts a night, a rested one a nap.
pub fn fall_asleep(s: &mut GameState) -> Result<(), Refusal> {
    if s.is_sleeping {
        return Err(Refusal::Asleep);
    }
    s.night = (s.energy as i32) < lvl(t::DOSE_NIGHT_ENERGY_PCT());
    s.is_sleeping = true;
    s.sleep_min = 0;
    s.good_sleep_min = 0;
    Ok(())
}

/// The player wakes the pet (grumpy under the engine's grumpy line).
pub fn wake_up(s: &mut GameState) -> Result<(), Refusal> {
    if !s.is_sleeping {
        return Err(Refusal::NoNeed);
    }
    super::advance::wake(s, true);
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
    let alcoholic = drink.is_alcoholic();
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
