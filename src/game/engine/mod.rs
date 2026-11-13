//! Cyber Ægg game engine — delta-T progression with boundary-based wake
//! scheduling.
//!
//! Instead of ticking every 10 seconds, the engine:
//! 1. Computes elapsed ticks since the last update.
//! 2. Applies all stat changes for that delta in one step.
//! 3. Computes the next boundary crossing time across all stats.
//! 4. Schedules a wake-up at the earliest boundary (or MAX_SLEEP_TICKS()).
//!
//! This lets the CPU sleep for minutes or hours when nothing interesting
//! is about to happen, saving significant battery on the badge.

pub mod advance;
pub mod anim_files;
pub mod drink;
pub mod food;
pub mod needs;
pub mod regime;
#[cfg(feature = "simulator")]
pub mod sim;
pub mod solver;
pub mod thresholds;
pub mod to_display;

use needs::{MAX, P};
use thresholds::*;
pub use drink::DrinkKind;
pub use food::FoodKind;
pub use to_display::DisplayAnim;

/// Engine ticks per simulated minute.
const TICKS_PER_MINUTE: u32 = 6;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Kind of pet — determines which sprite set to use.
///
/// Pets are not enumerated in code: the roster is whatever `PETS.CFG`
/// declares.  Adding one means dropping its `PPAAFF.PCX` sprites on the USB
/// partition and adding a manifest line — the filename generation in
/// `anim_files` and the selection screen pick it up automatically.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
///
/// Represented as a single byte — the sprite-prefix (`PP` in `PPAAFF.PCX`).
/// Which bytes name a real pet is decided at runtime by
/// [`crate::game::pet_registry`], populated from the `PETS.CFG` manifest.
/// Persisted as this byte, so saves round-trip regardless of the roster.
#[repr(transparent)]
pub struct PetKind(pub u8);

impl PetKind {
    /// Sprite prefix `0` — the placeholder used where a kind is structurally
    /// required but none has been chosen: pre-`pet_kind` saves, empty realm
    /// slots, and an unresolvable battle opponent.  Carries no guarantee of
    /// being in the roster; `PETS.CFG` decides whether prefix 0 is a pet.
    pub const DEFAULT: PetKind = PetKind(0);

    /// Reconstruct from the persisted / sprite-prefix byte.  Any byte is a
    /// valid id; whether it resolves to a real pet is decided by the registry.
    pub fn from_u8(v: u8) -> Self {
        PetKind(v)
    }

    /// The sprite-prefix / persisted byte.
    pub fn id(self) -> u8 {
        self.0
    }

    /// Human-readable name for the selection screen and Unicorn Realm.
    /// Resolves through the runtime registry populated from `PETS.CFG`.
    pub fn name(self) -> &'static str {
        crate::game::pet_registry::name_of(self.0)
    }

    /// All selectable pet kinds, in `PETS.CFG` order.  Empty until the
    /// manifest is installed, and on a badge whose manifest declares none.
    pub fn roster() -> &'static [PetKind] {
        crate::game::pet_registry::roster()
    }
}

/// Lifecycle phase of the pet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum Phase {
    /// Waiting to hatch (countdown running).
    Hatching,
    /// Alive and active.
    Active,
    /// Pet is leaving (countdown running).
    Leaving,
    /// Pet has left — ready to start a new egg.
    Gone,
}

/// HAX earned by the "Only pets" hobby (non-broke) branch.
pub const ONLYPETS_HOBBY_REWARD: u32 = 20;
/// HAX earned for winning a mini-game.
pub const MINIGAME_HAX_REWARD: u32 = 15;
/// HAX earned for winning a mesh Battle.
pub const BATTLE_HAX_REWARD: u32 = 20;
/// Happiness change per Play / Only-pets completion = 30% of STAT_MAX (65535).
pub const HAPPINESS_STEP: u16 = 19660;
/// HAX cost of the basic Play action.
pub const PLAY_HAX_COST: u32 = 10;
/// HAX cost of a drug dose (Ozempic / Medicate / Rehab).
pub const DRUG_HAX_COST: u32 = 15;
/// HAX cost of an Aspirine (the Heal action).
pub const ASPIRINE_HAX_COST: u32 = 1;
/// Hard-mode multiplier on medication (Insulin / Ozempic).
pub const MEDICATION_HARD_MULT: u32 = 3;
/// Hard-mode multiplier on Rehab.
pub const REHAB_HARD_MULT: u32 = 5;

/// HAX cost of a medication dose (Insulin / Ozempic) in the current mode.
pub fn medication_price(hard: bool) -> u32 {
    DRUG_HAX_COST * if hard { MEDICATION_HARD_MULT } else { 1 }
}

/// HAX cost of Rehab in the current mode.
pub fn rehab_price(hard: bool) -> u32 {
    DRUG_HAX_COST * if hard { REHAB_HARD_MULT } else { 1 }
}
/// Below this balance the pet is "broke" — Only-pets forces the low-pay,
/// happiness-draining work branch instead of the hobby branch.
pub const BROKE_THRESHOLD: u32 = 20;
/// HAX from the Only-pets BROKE (forced-work) branch.
pub const ONLYPETS_BROKE_REWARD: u32 = 100;

/// Active user action (mutually exclusive).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum Action {
    Feed,
    Heal,
    Play,
    Exercise,
    Medicate,
    /// Accelerated weight loss — not gated on being diabetic, unlike
    /// `Medicate` (insulin).
    Ozempic,
    Drink,
    /// Treatment for alcoholism — gated on `alcoholic`, mirrors
    /// `Medicate`'s relationship to diabetic.
    Rehab,
    /// "Only pets" work/hobby action — earns HAX. Only reachable when
    /// `money_enabled`. See `GameState::only_pets`.
    OnlyPets,
}

impl Action {
    /// Persisted discriminant. Explicit (not a bare `as u8` cast) and
    /// paired 1:1 with `from_u8` right below so adding a new variant is
    /// a visible two-place edit — `to_bytes`/`from_bytes` previously
    /// used an `as u8` cast on the write side but a hand-written match
    /// on the read side that only covered the first 4 variants,
    /// silently discarding an in-progress Exercise/Medicate/Ozempic/
    /// Drink/Rehab action on every reboot that landed mid-action.
    ///
    /// `2` is a deliberate gap — it used to be `Relax`, removed along
    /// with the `drained` stat it existed to relieve. Left unassigned
    /// (rather than renumbering Play onward) so old persisted bytes for
    /// the other actions keep meaning across the removal.
    #[allow(dead_code)] // used again by the v3 save format (task 11)
    fn to_u8(self) -> u8 {
        match self {
            Action::Feed => 0,
            Action::Heal => 1,
            Action::Play => 3,
            Action::Exercise => 4,
            Action::Medicate => 5,
            Action::Ozempic => 6,
            Action::Drink => 7,
            Action::Rehab => 8,
            Action::OnlyPets => 9,
        }
    }

    #[allow(dead_code)] // used again by the v3 save format (task 11)
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Action::Feed),
            1 => Some(Action::Heal),
            2 => None, // formerly Relax — see `to_u8`.
            3 => Some(Action::Play),
            4 => Some(Action::Exercise),
            5 => Some(Action::Medicate),
            6 => Some(Action::Ozempic),
            7 => Some(Action::Drink),
            8 => Some(Action::Rehab),
            9 => Some(Action::OnlyPets),
            _ => None,
        }
    }
}

/// Mini-games each track their own post-win cooldown so winning one
/// doesn't lock the player out of the others — nudges variety.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum MiniGame {
    TicTacToe,
    LightsOut,
    BlackHole,
    Nim,
    BornJeweled,
}

/// The complete game state.  Serialisable to ekv for save/restore.
#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    // Pet kind.
    pub pet_kind: PetKind,

    // Needs. v3 levels are good-high (u16 scale, `needs::P` = one percent).
    pub fullness: u16,
    pub energy: u16,
    pub health: u16,
    pub mood: u16,
    /// Body mass: 50 % is ideal, both ends are unhealthy.
    pub weight: u16,
    pub booze: u16,
    pub hydration: u16,
    pub fun: u16,
    pub strain: u16,
    pub dirt: u16,
    pub fitness: u16,

    // Minute timers (1 engine minute = 6 ticks).
    pub sleep_min: u16,
    pub oversleep_min: u16,
    pub good_sleep_min: u16,
    pub rested_min: u16,
    pub injured_min: u16,
    pub groggy_min: u16,
    pub hangover_min: u16,
    pub rehab_min: u16,
    pub recovering_min: u16,
    pub junk_min: u16,
    pub ozempic_min: u16,
    pub med_cd_min: u16,
    pub insulin_min: u16,
    pub ozempic_cd_min: u16,
    pub reversal_min: u16,

    // Flags and small counters.
    pub booze_peak: bool,
    pub deprived: bool,
    pub night: bool,
    pub no_groggy: bool,
    pub short_nights: u8,
    pub good_nights: u8,
    /// Ticks left over when `update` converts ticks to whole minutes.
    pub tick_rem: u8,
    pub care_good_min: u32,
    pub care_awake_min: u32,

    // Diabetes — permanent once triggered by sustained overweight.
    pub diabetic: bool,
    /// Minutes spent overweight; once it reaches the onset time `diabetic`
    /// flips true for the rest of this pet's life.
    /// Counted in quarter-minutes (`advance::SLOW_SCALE`).
    pub diabetes_min: u32,

    // Dependence — permanent once triggered by sustained drinking.
    pub dependent: bool,
    /// Counted in quarter-minutes (`advance::SLOW_SCALE`): +4 per minute
    /// while drunk, -1 per minute while sober.
    pub dependence_min: u32,

    // Traits (higher = better).
    pub vitality: u16,
    pub curiosity: u16,
    pub resilience: u16,

    // Timing.
    pub last_update_tick: u32,
    pub age_ticks: u32,

    // Lifecycle.
    pub phase: Phase,
    pub hatching_countdown: u16,
    /// Leaving countdown in half-minutes (`advance::LEAVING_SCALE`): +2 per
    /// minute at health 0, -1 per minute otherwise; Gone at 720 minutes.
    pub leaving_min: u32,
    pub generation: u16,

    // Mesh Battle record — lifetime counters, never reset by anything
    // other than a fresh egg (see `new_egg`/`new_generation`).
    pub wins: u16,
    pub losses: u16,

    // HAX currency — belongs to the current pet, resets with a new egg.
    /// HAX balance. New egg starts at 100.
    pub money: u32,
    /// Chosen at pet creation, persisted. When `false` the whole money
    /// layer is inert: no HAX display, no prices, no rewards.
    pub money_enabled: bool,
    /// Chosen at pet creation, persisted. Only meaningful when
    /// `money_enabled` is true (hard mode implies money is on) — changes
    /// PRICE AMOUNTS only (healthy food, medication, rehab all cost
    /// more); it never changes which actions are gated on affordability,
    /// that's still entirely `money_enabled`'s job.
    pub hard_mode: bool,

    // Action state.
    pub active_action: Option<Action>,
    /// Which food is being eaten during an in-progress `Action::Feed`.
    /// Transient (not persisted) — only meaningful mid-action; defaults
    /// to `FoodKind::Apple`-equivalent multipliers if `None` (e.g. a
    /// reboot mid-feed for the remaining tick or two of that action).
    pub active_food: Option<FoodKind>,
    /// Which drink is being drunk during an in-progress `Action::Drink`.
    /// Same transient/not-persisted treatment as `active_food`.
    pub active_drink: Option<DrinkKind>,
    pub action_ticks_remaining: u8,
    /// Cooldown between mesh Battles — resolves instantly (no in-progress
    /// duration), so this is the only battle-related field that behaves
    /// like the other primary-action cooldowns above.
    pub cooldown_battle: u16,
    /// Per-mini-game cooldown after winning.  Each game tracks its
    /// own counter so winning one doesn't gate the others.  None of
    /// these are persisted to flash — rebooting clears them.
    pub cooldown_tictactoe: u16,
    pub cooldown_lightsout: u16,
    pub cooldown_blackhole: u16,
    pub cooldown_nim: u16,
    pub cooldown_bornjeweled: u16,
    /// Cooldown after completing "Only pets". Transient (not persisted),
    /// same policy as the mini-game cooldowns above — rebooting mid-cooldown
    /// just makes the action available again a little early.
    pub cooldown_onlypets: u16,

    // Sleep.
    pub is_sleeping: bool,

    // Hibernation — all progression frozen, time stands still.
    pub hibernating: bool,
    /// Total ticks spent in hibernation during this pet's life.
    pub hibernate_ticks: u32,

    // Persistence — tracks when state was last saved to flash.
    /// `age_ticks` at the time of the last successful save.
    /// Not part of the game logic — only used by the save system.
    last_save_tick: u32,

    /// Transient flag (not serialized): when true, the next
    /// `needs_save()` check returns true regardless of the interval.
    /// Set on game start and phase transitions so the save happens
    /// immediately rather than waiting 15 minutes.
    save_pending: bool,

    /// Transient flag (not serialized): set when the pet transitions to
    /// Gone, so lifecycle can record it in the Unicorn Realm.
    pub realm_pending: bool,

    /// Transient flag (not serialized): set when hatching completes,
    /// so lifecycle can prompt the player to name their pet.
    pub naming_pending: bool,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl GameState {
    /// Create a new egg with randomised traits from a seed.
    pub fn new_egg(seed: u64, kind: PetKind) -> Self {
        // Simple xorshift64 for deterministic trait generation.
        let mut rng = seed;
        let mut next = || -> u16 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let range = (MAX_TRAIT() - MIN_TRAIT()) as u64;
            MIN_TRAIT() + ((rng % range) as u16)
        };

        let vitality = next();
        let curiosity = next();
        let resilience = next();

        Self {
            pet_kind: kind,

            fullness: 60 * P as u16,
            energy: 80 * P as u16,
            health: (85 * P - (MAX - vitality as i32) / 8) as u16,
            mood: 70 * P as u16,
            weight: 50 * P as u16,
            booze: 0,
            hydration: 65 * P as u16,
            fun: 50 * P as u16,
            strain: 0,
            dirt: 10 * P as u16,
            fitness: 60 * P as u16,

            sleep_min: 0,
            oversleep_min: 0,
            good_sleep_min: 0,
            rested_min: 0,
            injured_min: 0,
            groggy_min: 0,
            hangover_min: 0,
            rehab_min: 0,
            recovering_min: 0,
            junk_min: 0,
            ozempic_min: 0,
            med_cd_min: 0,
            insulin_min: 0,
            ozempic_cd_min: 0,
            reversal_min: 0,

            booze_peak: false,
            deprived: false,
            night: false,
            no_groggy: false,
            short_nights: 0,
            good_nights: 0,
            tick_rem: 0,
            care_good_min: 0,
            care_awake_min: 0,

            diabetic: false,
            diabetes_min: 0,

            dependent: false,
            dependence_min: 0,

            vitality,
            curiosity,
            resilience,

            last_update_tick: 0,
            age_ticks: 0,

            phase: Phase::Hatching,
            hatching_countdown: HATCHING_TICKS(),
            leaving_min: 0,
            generation: 0,

            wins: 0,
            losses: 0,

            money: 100,
            money_enabled: true,
            hard_mode: false,

            active_action: None,
            active_food: None,
            active_drink: None,
            action_ticks_remaining: 0,
            cooldown_battle: 0,
            cooldown_tictactoe: 0,
            cooldown_lightsout: 0,
            cooldown_blackhole: 0,
            cooldown_nim: 0,
            cooldown_bornjeweled: 0,
            cooldown_onlypets: 0,

            is_sleeping: false,
            hibernating: false,
            hibernate_ticks: 0,
            last_save_tick: 0,
            save_pending: true,
            realm_pending: false,
            naming_pending: false,
        }
    }

    /// Start a new generation (pet left, hatch new egg).
    pub fn new_generation(&mut self, seed: u64, kind: PetKind) {
        let next_gen = self.generation + 1;
        *self = Self::new_egg(seed, kind);
        self.generation = next_gen;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Delta-T update
// ---------------------------------------------------------------------------

impl GameState {
    /// Advance the game state to `now_tick`.
    ///
    /// Time moves in whole engine minutes (1 minute = 6 ticks); the sub-minute
    /// remainder is carried in `tick_rem`.  A backwards clock counts as zero
    /// elapsed.  Each loop pass advances one solver segment, so a long gap
    /// costs a handful of closed-form steps, not one per minute.
    pub fn update(&mut self, now_tick: u32) {
        let total_delta = now_tick.saturating_sub(self.last_update_tick);
        if total_delta == 0 {
            return;
        }
        self.last_update_tick = now_tick;

        // Hibernation: time stands still.  Track hibernated time but
        // don't advance age or any game state.
        if self.hibernating {
            self.hibernate_ticks += total_delta;
            return;
        }

        if self.phase == Phase::Gone {
            return;
        }

        self.age_ticks += total_delta;

        match self.phase {
            Phase::Gone => unreachable!(),
            Phase::Hatching => {
                let consumed = total_delta.min(self.hatching_countdown as u32);
                self.hatching_countdown -= consumed as u16;
                if self.hatching_countdown == 0 {
                    self.phase = Phase::Active;
                    self.save_pending = true;
                    self.naming_pending = true;
                }
                return;
            }
            Phase::Leaving | Phase::Active => {}
        }

        // A running action eats its ticks first (v2 effects land in a later
        // task); without one nothing is consumed.
        let mut ticks = total_delta + self.tick_rem as u32;
        if self.active_action.is_some() {
            ticks = self.consume_action_ticks(ticks);
        }
        let mut minutes = ticks / TICKS_PER_MINUTE;
        self.tick_rem = (ticks % TICKS_PER_MINUTE) as u8;
        while minutes > 0 {
            let mut dt = solver::seg_len(self, minutes).clamp(1, minutes);
            // Mood and health crossing a line of their own is not bounded by
            // the solver: halve the segment while a trial advance changes the
            // signature. At most log2(dt) trials, each a closed-form step on a
            // copy, so a long gap on the badge still costs a handful of steps.
            let before = solver::sig(self);
            while dt > 1 {
                let mut trial = self.clone();
                advance::advance(&mut trial, dt);
                // A target moving faster than mood can follow also needs
                // shorter segments (R22).
                let drift = (regime::mood_target(&trial) - regime::mood_target(self)).unsigned_abs();
                if solver::sig(&trial) == before && drift <= thresholds::MOOD_STEP() as u32 * dt {
                    break;
                }
                dt = dt.div_ceil(2);
            }
            advance::advance(self, dt);
            minutes -= dt;
        }
    }

    /// Consume action ticks from delta, applying action effects.
    /// Returns remaining delta after action ticks are consumed.
    fn consume_action_ticks(&mut self, delta: u32) -> u32 {
        let _ = delta;
        todo!("BornPets v2 plan 1: v2 action effects (later task)")
    }
}

// ---------------------------------------------------------------------------
// User actions
// ---------------------------------------------------------------------------

impl GameState {
    /// True when the pet is alive (Active or Leaving) and can receive actions.
    fn is_alive(&self) -> bool {
        self.phase == Phase::Active || self.phase == Phase::Leaving
    }

    /// Shared gate for every timed action starter: the pet must be alive and
    /// awake, no action running, and this action's cooldown expired. Then,
    /// when money is enabled, the optional `price` is charged (only after
    /// every other check passed, so a rejected action never costs HAX; an
    /// unaffordable one is rejected outright, see `spend_money`). On success
    /// the action and its duration are set. Callers with extra state
    /// (`active_food`, `active_drink`) set it after a `true` return.
    fn start_action(
        &mut self,
        action: Action,
        cooldown: u16,
        price: Option<u32>,
        duration: u8,
    ) -> bool {
        if !self.is_alive() || self.is_sleeping {
            return false;
        }
        if self.active_action.is_some() || cooldown > 0 {
            return false;
        }
        if let Some(price) = price
            && self.money_enabled
            && !self.spend_money(price)
        {
            return false; // can't afford — action rejected
        }
        self.active_action = Some(action);
        self.action_ticks_remaining = duration;
        true
    }

    /// Start the feed action with the chosen food.  Returns false if not
    /// available.
    pub fn feed(&mut self, food: FoodKind) -> bool {
        let price = food.hax_price(self.hard_mode);
        if !self.start_action(Action::Feed, 0, Some(price), FEED_DURATION()) {
            return false;
        }
        self.active_food = Some(food);
        true
    }

    /// Start the heal action (Aspirine costs HAX).
    pub fn heal(&mut self) -> bool {
        self.start_action(
            Action::Heal,
            0,
            Some(ASPIRINE_HAX_COST),
            HEAL_DURATION(),
        )
    }

    /// Start the play action.
    pub fn play(&mut self) -> bool {
        self.start_action(
            Action::Play,
            0,
            Some(PLAY_HAX_COST),
            PLAY_DURATION(),
        )
    }

    /// Send the pet to "Only pets" to earn HAX. Timed action like Play.
    pub fn only_pets(&mut self) -> bool {
        self.start_action(Action::OnlyPets, self.cooldown_onlypets, None, PLAY_DURATION())
    }

    /// Start the exercise action — the primary relief valve for `weight`.
    pub fn exercise(&mut self) -> bool {
        self.start_action(
            Action::Exercise,
            0,
            None,
            EXERCISE_DURATION(),
        )
    }

    /// Administer diabetes medication.  Only meaningful once `diabetic`
    /// is set — resets `cooldown_medicate`, which doubles as the
    /// protection window during which the diabetes sick-penalty is
    /// suppressed.
    pub fn medicate(&mut self) -> bool {
        if !self.diabetic {
            return false;
        }
        self.start_action(
            Action::Medicate,
            self.med_cd_min,
            Some(medication_price(self.hard_mode)),
            MEDICATE_DURATION(),
        )
    }

    /// Administer Ozempic — a stronger, faster-acting weight-loss
    /// treatment than Exercise. Unlike `medicate()`, this is *not*
    /// gated on being diabetic — any pet can take it.
    pub fn ozempic(&mut self) -> bool {
        self.start_action(
            Action::Ozempic,
            self.ozempic_cd_min,
            Some(medication_price(self.hard_mode)),
            OZEMPIC_DURATION(),
        )
    }

    /// Start the drink action with the chosen drink.  Returns false if
    /// not available.
    pub fn drink(&mut self, drink: DrinkKind) -> bool {
        if !self.start_action(
            Action::Drink,
            0,
            Some(drink.hax_price()),
            DRINK_DURATION(),
        ) {
            return false;
        }
        self.active_drink = Some(drink);
        true
    }

    /// Administer rehab treatment for alcoholism.  Only meaningful once
    /// `alcoholic` is set — mirrors `medicate()`.
    pub fn rehab(&mut self) -> bool {
        if !self.dependent {
            return false;
        }
        self.start_action(
            Action::Rehab,
            self.rehab_min,
            Some(rehab_price(self.hard_mode)),
            REHAB_DURATION(),
        )
    }

    /// Put the pet to sleep.
    pub fn sleep(&mut self) -> bool {
        if !self.is_alive() || self.is_sleeping {
            return false;
        }
        self.is_sleeping = true;
        true
    }

    /// Wake the pet up.
    pub fn wake(&mut self) -> bool {
        if !self.is_sleeping {
            return false;
        }
        self.is_sleeping = false;
        true
    }

    /// Hibernate the pet — all progression freezes.  Marks the
    /// state for immediate persistence so a power-off before the
    /// next 15-minute save tick still preserves the toggle.
    pub fn hibernate(&mut self) -> bool {
        if self.hibernating || self.phase == Phase::Gone {
            return false;
        }
        self.hibernating = true;
        self.request_save();
        true
    }

    /// End hibernation — progression resumes from this moment.
    /// Marks the state for immediate persistence (see `hibernate`).
    pub fn wake_from_hibernation(&mut self) -> bool {
        if !self.hibernating {
            return false;
        }
        self.hibernating = false;
        self.request_save();
        true
    }

    /// Award the mini-game win reward: HAX (when `money_enabled`) plus the
    /// per-game cooldown.  Starts only that game's cooldown so other
    /// mini-games stay available — encourages variety.  Also bumps
    /// hunger: playing burns calories.
    pub fn award_inspiration(&mut self, game: MiniGame) {
        if self.phase != Phase::Active {
            return;
        }
        self.fullness = self.fullness.saturating_sub(MINIGAME_HUNGER_COST());
        match game {
            MiniGame::TicTacToe => self.cooldown_tictactoe = MINIGAME_COOLDOWN(),
            MiniGame::LightsOut => self.cooldown_lightsout = MINIGAME_COOLDOWN(),
            MiniGame::BlackHole => self.cooldown_blackhole = MINIGAME_COOLDOWN(),
            MiniGame::Nim => self.cooldown_nim = MINIGAME_COOLDOWN(),
            MiniGame::BornJeweled => self.cooldown_bornjeweled = MINIGAME_COOLDOWN(),
        }
        if self.money_enabled {
            self.add_money(MINIGAME_HAX_REWARD);
        }
    }

    /// Total hours the pet has spent in hibernation during its life.
    pub fn hibernate_hours(&self) -> u32 {
        self.hibernate_ticks / 360 // 360 ticks = 1 hour
    }
}

// ---------------------------------------------------------------------------
// Debug cheats — reachable only via the hidden button sequence in
// `crate::game::debug_cheats`. Not part of normal gameplay; exist purely so
// the multi-day weight/diabetes arc can be tested in seconds instead of days.
// ---------------------------------------------------------------------------

impl GameState {
    /// Push `weight` just over the overweight trigger.
    pub fn debug_force_overweight(&mut self) {
        self.weight = (needs::weight_hi() as u16).saturating_add(1);
    }

    /// Flip `diabetic` on directly, skipping the sustained-overweight timer.
    pub fn debug_force_diabetic(&mut self) {
        self.diabetic = true;
    }

    /// Clear diabetes and the overweight-duration counter, so the arc can
    /// be re-tested from scratch without starting a new pet.
    pub fn debug_clear_diabetes(&mut self) {
        self.diabetic = false;
        self.diabetes_min = 0;
    }

    /// Fast-forward the engine by `ticks` in one shot — runs the same
    /// `update()` path real time would, just compressed. Used for "Skip
    /// 1 hour" / "Skip 1 day" cheat items.
    pub fn debug_skip_ticks(&mut self, ticks: u32) {
        let target = self.last_update_tick.saturating_add(ticks);
        self.update(target);
    }

    /// Push `drunk` just over the alcoholism trigger.
    pub fn debug_force_drunk(&mut self) {
        self.booze = (needs::booze_danger() as u16).saturating_add(1);
    }

    /// Flip `alcoholic` on directly, skipping the sustained-drunk timer.
    pub fn debug_force_alcoholic(&mut self) {
        self.dependent = true;
    }

    /// Clear alcoholism and the drunk-duration counter, so the arc can
    /// be re-tested from scratch without starting a new pet.
    pub fn debug_clear_alcoholism(&mut self) {
        self.dependent = false;
        self.dependence_min = 0;
    }

    /// Zero this pet's lifetime mesh Battle record and cooldown. Paired
    /// with `crate::game::friends::reset_all_battle_records` (which
    /// zeros the per-friend head-to-head numbers) by the
    /// `lifecycle::debug_reset_battle_record` wrapper — a badge that
    /// picked up inflated counts from a duplicate-delivery bug before it
    /// was fixed has no other way to get back to a clean baseline.
    pub fn debug_reset_battle_record(&mut self) {
        self.wins = 0;
        self.losses = 0;
        self.cooldown_battle = 0;
    }
}

// ---------------------------------------------------------------------------
// Next wake time — boundary-based scheduling
// ---------------------------------------------------------------------------

impl GameState {
    /// Compute the number of ticks until the next interesting event.
    ///
    /// Returns the absolute tick time to wake up at.  The caller should
    /// set a timer for `next_wake_tick - last_update_tick` ticks.
    pub fn next_wake_tick(&self) -> u32 {
        if self.phase == Phase::Gone || self.hibernating {
            return u32::MAX;
        }
        // Hatching countdown.
        if self.phase == Phase::Hatching {
            return self.last_update_tick + self.hatching_countdown as u32;
        }
        todo!("BornPets v2 plan 1: v2 wake scheduler (later task)")
    }
}

// ---------------------------------------------------------------------------
// PetStats — display-friendly snapshot (0 = bad, 100 = good)
// ---------------------------------------------------------------------------

/// Display-friendly snapshot of the pet's state.
///
/// All stats are scaled 0–100 with **positive semantics**: 100 = perfect,
/// 0 = critical.  This is the inverse of the internal u16 representation
/// where 0 = best.
///
/// Obtain via [`GameState::stats()`] which triggers a state update first.
/// The result is cached — calling `stats()` again at the same tick is free.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub struct PetStats {
    /// How well-fed the pet is (100 = full, 0 = starving).
    pub hunger: u8,
    /// How rested the pet is (100 = alert, 0 = exhausted).
    pub tired: u8,
    /// How healthy the pet is (100 = healthy, 0 = critically ill).
    pub healthy: u8,
    /// How happy the pet is (100 = happy, 0 = miserable).
    pub happy: u8,
    /// How lean/fit the pet is (100 = lean, 0 = obese).
    pub weight: u8,
    /// How sober the pet is (100 = stone sober, 0 = maximally drunk).
    pub drunk: u8,

    /// Whether the pet has developed type 2 diabetes (permanent once set).
    pub diabetic: bool,
    /// Whether weight is currently above the overweight trigger — the
    /// same condition that (if sustained long enough) leads to
    /// diabetes. Independent of `diabetic`, which is permanent once
    /// set: a pet can be overweight without (yet) being diabetic, or
    /// diabetic while currently back under the overweight line.
    pub overweight: bool,
    /// Whether the pet has developed alcoholism (permanent once set) —
    /// same relationship to `drunk` as `diabetic` has to `overweight`.
    pub alcoholic: bool,

    /// Current lifecycle phase.
    pub phase: Phase,
    /// Whether the pet is sleeping.
    pub is_sleeping: bool,
    /// Age in ticks (1 tick = 10 seconds).
    pub age_ticks: u32,
    /// Generation number (0 = first pet).
    pub generation: u16,

    /// Lifetime mesh Battle wins/losses — see `crate::game::battle`.
    pub wins: u16,
    pub losses: u16,

    /// Current HAX balance. Only meaningful (displayed/priced) when
    /// `money_enabled`.
    pub money: u32,
    /// Whether the money layer is active for this pet — gates HAX display,
    /// pricing, and menu affordability checks.
    pub money_enabled: bool,
    /// Whether Hard (US) prices are in effect. Only meaningful when
    /// `money_enabled` is true — see `GameState::hard_mode`.
    pub hard_mode: bool,

    /// Currently active action (if any).
    pub active_action: Option<Action>,
    /// Ticks remaining on the active action.
    pub action_ticks_remaining: u8,

    /// Action availability (true = can be started right now).
    pub can_feed: bool,
    pub can_heal: bool,
    pub can_play: bool,
    /// "Only pets" — same in-progress-action/cooldown gating as Play.
    /// Only ever surfaced in the menu when `money_enabled`, but this flag
    /// itself doesn't know about money — the menu gates visibility.
    pub can_only_pets: bool,
    pub can_exercise: bool,
    /// Only true once `diabetic` is set — administering medication to a
    /// non-diabetic pet is a no-op.
    pub can_medicate: bool,
    /// Unlike `can_medicate`, not gated on diabetic status.
    pub can_ozempic: bool,
    pub can_drink: bool,
    /// Only true once `alcoholic` is set — mirrors `can_medicate`.
    pub can_rehab: bool,
    pub can_sleep: bool,
    pub can_wake: bool,
    /// Per-mini-game availability.  False while that game's post-win
    /// cooldown is running; the others stay independently playable.
    pub can_play_tictactoe: bool,
    pub can_play_lightsout: bool,
    pub can_play_blackhole: bool,
    pub can_play_nim: bool,
    pub can_play_bornjeweled: bool,
    /// Whether a mesh Battle can be started right now (cooldown-gated,
    /// same shape as the mini-game `can_play_*` flags above). Does not
    /// factor in whether any friends exist — the Battle screen handles
    /// the "no friends yet" empty state itself.
    pub can_battle: bool,

    /// Remaining action cooldowns in ticks (1 tick = 10 s).  0 = ready.
    /// Mirrored from the matching `GameState` fields so the modal can
    /// show the exact remaining time on a disabled menu row.
    pub cooldown_feed: u16,
    pub cooldown_heal: u16,
    pub cooldown_play: u16,
    pub cooldown_onlypets: u16,
    pub cooldown_exercise: u16,
    pub cooldown_medicate: u16,
    pub cooldown_ozempic: u16,
    pub cooldown_drink: u16,
    pub cooldown_rehab: u16,
    pub cooldown_tictactoe: u16,
    pub cooldown_lightsout: u16,
    pub cooldown_blackhole: u16,
    pub cooldown_nim: u16,
    pub cooldown_bornjeweled: u16,
    pub cooldown_battle: u16,

    /// Whether the pet is hibernating (all progression frozen).
    pub hibernating: bool,
    /// Total hours spent in hibernation during this pet's life.
    pub hibernate_hours: u32,
}

/// A stored pet name (`buf`, `len` bytes used) as a str. `len` may come from
/// flash and be corrupt, so it is clamped to the buffer; invalid UTF-8 gives
/// an empty name.
pub fn name_str(buf: &[u8], len: u8) -> &str {
    let n = (len as usize).min(PET_NAME_MAX).min(buf.len());
    core::str::from_utf8(&buf[..n]).unwrap_or("")
}

/// Ticks (1 tick = 10 s) as "Xd Xh".
pub fn fmt_days_hours(ticks: u32) -> heapless::String<12> {
    let hours = ticks / 360;
    let days = hours / 24;
    let mut s = heapless::String::new();
    let _ = core::fmt::Write::write_fmt(&mut s, format_args!("{}d {}h", days, hours % 24));
    s
}

/// A trait value (0..=65535, higher = stronger) as a 0..=100 percentage.
/// Not the inverted [`to_display_pct`], which maps "bad" stats.
pub fn trait_pct(v: u16) -> u8 {
    (v as u32 * 100 / 65535) as u8
}

/// Convert internal stat (0=good, 65535=bad) to display (0=bad, 100=good).
#[allow(dead_code)] // used again by the v3 `stats()` (task 10)
fn to_display_pct(val: u16) -> u8 {
    100 - (val as u32 * 100 / STAT_MAX() as u32) as u8
}

impl GameState {
    /// Update the game state to `now_tick` and return a display snapshot.
    ///
    /// This is the primary API for the UI layer.  It triggers a state
    /// update, then returns the snapshot.  The result is cached internally —
    /// calling `stats()` again at the same tick skips the update and
    /// returns the cached snapshot.
    pub fn stats(&mut self, now_tick: u32) -> PetStats {
        let _ = now_tick;
        todo!("BornPets v2 plan 1 task 10: PetStats from v3 fields")
    }
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

impl GameState {
    /// Returns `true` if the state should be saved to flash.
    ///
    /// Becomes true when at least `SAVE_INTERVAL_TICKS()` (15 minutes)
    /// have elapsed since the last save.  The caller does the async
    /// save and then calls `mark_saved()`.
    ///
    /// No extra wake-ups are scheduled for saving — this check
    /// piggybacks on whatever triggered the current update.
    pub fn needs_save(&self) -> bool {
        self.save_pending
            || self.age_ticks.saturating_sub(self.last_save_tick) >= SAVE_INTERVAL_TICKS()
    }

    /// Mark the state as successfully saved.  Resets the save timer.
    pub fn mark_saved(&mut self) {
        self.last_save_tick = self.age_ticks;
        self.save_pending = false;
    }

    /// Request that the next `save_if_needed()` persists this state
    /// immediately, rather than waiting for the next 15-minute interval.
    pub fn request_save(&mut self) {
        self.save_pending = true;
    }

    /// Reduce `miserable` from meeting (or spending time with) a mesh
    /// friend over the SHDW channel — see `crate::game::friends`.
    /// `big` is a bigger one-time bump for a brand-new friend; a smaller
    /// bump applies to a cooldown-gated recurring reunion with someone
    /// already known.  Just another `miserable` reduction, so it's
    /// naturally re-clamped by `apply_miserable_floor()` on the next tick
    /// if the pet is Leaving/critical, the same way `Play` already works.
    pub fn friend_boost(&mut self, big: bool) {
        // STAT_MAX() is 65535, so `* 2` overflows u16 before the `/ 5` can
        // bring it back down — widen to u32 for the multiply.
        let amount = if big {
            (STAT_MAX() as u32 * 2 / 5) as u16 // ~40%
        } else {
            STAT_MAX() / 10 // ~10%
        };
        self.mood = self.mood.saturating_add(amount);
    }

    /// Record the outcome of a mesh Battle — see `crate::game::battle`.
    /// Purely a lifetime win/loss tally plus the cooldown gate; battle HP
    /// itself is never persisted or connected to any real stat, so a loss
    /// here has no effect on the pet's actual health/lifecycle.
    pub fn record_battle(&mut self, won: bool) {
        if won {
            self.wins = self.wins.saturating_add(1);
        } else {
            self.losses = self.losses.saturating_add(1);
        }
        self.cooldown_battle = BATTLE_COOLDOWN();
        if won && self.money_enabled {
            self.add_money(BATTLE_HAX_REWARD);
        }
    }

    /// Credit `delta` HAX, saturating at `u32::MAX`. Immediately flags the
    /// state for save (see `request_save`) rather than waiting for the
    /// next 15-minute save interval — same immediate-persist pattern as
    /// pet name / hibernate, since a HAX balance is worth protecting from
    /// an unlucky reboot right after being earned.
    pub fn add_money(&mut self, delta: u32) {
        self.money = self.money.saturating_add(delta);
        self.request_save();
    }

    /// Debit `price` HAX if affordable. Returns `true` and flags the state
    /// for immediate save on success; returns `false` and leaves `money`
    /// unchanged (no save) if the balance is insufficient. Callers are
    /// expected to have already gated the action on `can_afford()` so this
    /// should normally succeed, but the check is repeated here so the
    /// balance can never go negative.
    pub fn spend_money(&mut self, price: u32) -> bool {
        if self.money >= price {
            self.money -= price;
            self.request_save();
            true
        } else {
            false
        }
    }

    /// Whether the current balance covers `price`.
    pub fn can_afford(&self, price: u32) -> bool {
        self.money >= price
    }
}

// ---------------------------------------------------------------------------
// Serialization — manual, no serde, fixed-size
// ---------------------------------------------------------------------------

/// Format version stamped into byte 0 of every save. Bump this whenever
/// the field layout below changes so an old-layout blob from a prior
/// firmware fails the guard in `from_bytes` (→ clean fresh egg) instead
/// of being silently reinterpreted with the new offsets. v1 was the
/// original unversioned 96-byte layout; v2 adds this byte + HAX money.
pub const SAVE_FORMAT_VERSION: u8 = 2;

/// Serialized size of GameState in bytes (1 version byte + 96 fields).
pub const SAVE_SIZE: usize = 97;

impl GameState {
    /// Serialize the game state to a fixed-size byte buffer for ekv.
    #[allow(unused_assignments)]
    pub fn to_bytes(&self) -> [u8; SAVE_SIZE] {
        todo!("BornPets v2 plan 1 task 11: v3 save format")
    }

    /// Deserialize a game state from a byte buffer.
    /// Returns `None` if the buffer is too short.
    #[allow(unused_assignments)]
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        let _ = b;
        todo!("BornPets v2 plan 1 task 11: v3 save format")
    }
}

// ---------------------------------------------------------------------------
// Unicorn Realm — past pet records
// ---------------------------------------------------------------------------

/// Maximum length of a pet name in bytes.
pub const PET_NAME_MAX: usize = 12;

/// Compact record of a past pet for the Unicorn Realm.
#[derive(Clone, Copy)]
pub struct PetRecord {
    pub generation: u16,
    pub age_ticks: u32,
    pub vitality: u16,
    pub curiosity: u16,
    pub resilience: u16,
    pub pet_kind: PetKind,
    /// Pet name (UTF-8, up to PET_NAME_MAX bytes, zero-padded).
    pub name: [u8; PET_NAME_MAX],
    pub name_len: u8,
}

/// Serialized size of one PetRecord (12 data + 1 kind + 12 name + 1 name_len).
pub const PET_RECORD_SIZE: usize = 26;
/// Maximum number of past pets stored.
pub const REALM_MAX_PETS: usize = 10;
/// Total serialized size: 1 byte count + N records.
pub const REALM_SAVE_SIZE: usize = 1 + REALM_MAX_PETS * PET_RECORD_SIZE;

impl PetRecord {
    /// Create a record from the current game state and name (call when pet
    /// leaves).
    pub fn from_game_state(state: &GameState, pet_name: &[u8]) -> Self {
        let mut name = [0u8; PET_NAME_MAX];
        let len = pet_name.len().min(PET_NAME_MAX);
        name[..len].copy_from_slice(&pet_name[..len]);
        Self {
            generation: state.generation,
            age_ticks: state.age_ticks,
            vitality: state.vitality,
            curiosity: state.curiosity,
            resilience: state.resilience,
            pet_kind: state.pet_kind,
            name,
            name_len: len as u8,
        }
    }

    fn to_bytes(self, buf: &mut [u8]) {
        buf[0..2].copy_from_slice(&self.generation.to_le_bytes());
        buf[2..6].copy_from_slice(&self.age_ticks.to_le_bytes());
        buf[6..8].copy_from_slice(&self.vitality.to_le_bytes());
        buf[8..10].copy_from_slice(&self.curiosity.to_le_bytes());
        buf[10..12].copy_from_slice(&self.resilience.to_le_bytes());
        buf[12] = self.pet_kind.0;
        buf[13..25].copy_from_slice(&self.name);
        buf[25] = self.name_len;
    }

    fn from_bytes(buf: &[u8]) -> Self {
        let mut name = [0u8; PET_NAME_MAX];
        let pet_kind = if buf.len() >= 26 {
            name.copy_from_slice(&buf[13..25]);
            PetKind::from_u8(buf[12])
        } else if buf.len() >= 25 {
            // Old 25-byte format without pet_kind.
            name.copy_from_slice(&buf[12..24]);
            PetKind::DEFAULT
        } else {
            PetKind::DEFAULT
        };
        Self {
            generation: u16::from_le_bytes([buf[0], buf[1]]),
            age_ticks: u32::from_le_bytes([buf[2], buf[3], buf[4], buf[5]]),
            vitality: u16::from_le_bytes([buf[6], buf[7]]),
            curiosity: u16::from_le_bytes([buf[8], buf[9]]),
            resilience: u16::from_le_bytes([buf[10], buf[11]]),
            pet_kind,
            name,
            name_len: if buf.len() >= 26 {
                buf[25]
            } else if buf.len() >= 25 {
                buf[24]
            } else {
                0
            },
        }
    }

    /// Pet name as a str.
    pub fn name_str(&self) -> &str {
        // name_len comes from flash and could be corrupt; `name_str` clamps it
        // so the realm screen can't panic on a bad record.
        name_str(&self.name, self.name_len)
    }

    /// Format age as "Xd Xh".
    pub fn age_str(&self) -> heapless::String<12> {
        fmt_days_hours(self.age_ticks)
    }
}

/// Ring buffer of past pet records, newest first.
pub struct PetRealm {
    pub pets: [PetRecord; REALM_MAX_PETS],
    pub count: u8,
}

impl Default for PetRealm {
    fn default() -> Self {
        Self::new()
    }
}

impl PetRealm {
    pub const fn new() -> Self {
        Self {
            pets: [PetRecord {
                generation: 0,
                age_ticks: 0,
                vitality: 0,
                curiosity: 0,
                resilience: 0,
                pet_kind: PetKind::DEFAULT,
                name: [0; PET_NAME_MAX],
                name_len: 0,
            }; REALM_MAX_PETS],
            count: 0,
        }
    }

    /// Add a pet record, shifting older entries down. Drops the oldest if full.
    pub fn push(&mut self, record: PetRecord) {
        // Shift everything down by one.
        for i in (1..REALM_MAX_PETS).rev() {
            self.pets[i] = self.pets[i - 1];
        }
        self.pets[0] = record;
        if (self.count as usize) < REALM_MAX_PETS {
            self.count += 1;
        }
    }

    pub fn to_bytes(&self) -> [u8; REALM_SAVE_SIZE] {
        let mut buf = [0u8; REALM_SAVE_SIZE];
        buf[0] = self.count;
        for i in 0..self.count as usize {
            let offset = 1 + i * PET_RECORD_SIZE;
            self.pets[i].to_bytes(&mut buf[offset..offset + PET_RECORD_SIZE]);
        }
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Self {
        let mut realm = Self::new();
        if buf.is_empty() {
            return realm;
        }
        realm.count = buf[0].min(REALM_MAX_PETS as u8);
        for i in 0..realm.count as usize {
            let offset = 1 + i * PET_RECORD_SIZE;
            if offset + PET_RECORD_SIZE <= buf.len() {
                realm.pets[i] = PetRecord::from_bytes(&buf[offset..]);
            }
        }
        realm
    }
}

#[cfg(test)]
mod overweight_diabetes_tests {
    use super::*;

    /// New fields round-trip through `to_bytes`/`from_bytes` at the
    /// versioned 97-byte `SAVE_SIZE` (1 version byte + 96 fields).
    #[test]
    #[ignore = "v1 rules; replaced by BornPets v2 plan 1 task 11"]
    fn save_round_trip_includes_new_fields() {
        let mut state = GameState::new_egg(42, PetKind::from_u8(1));
        state.weight = 41000;
        state.diabetic = true;
        state.rehab_min = 12;
        state.med_cd_min = 345;
        state.wins = 3;
        state.losses = 1;
        state.cooldown_battle = 77;
        state.money = 12345;
        state.money_enabled = false;
        state.hard_mode = true;

        let bytes = state.to_bytes();
        assert_eq!(bytes.len(), SAVE_SIZE);
        assert_eq!(SAVE_SIZE, 97);
        assert_eq!(bytes[0], SAVE_FORMAT_VERSION);

        let restored = GameState::from_bytes(&bytes).expect("valid save should parse");
        assert_eq!(restored.weight, 41000);
        assert!(restored.diabetic);
        assert_eq!(restored.rehab_min, 12);
        assert_eq!(restored.med_cd_min, 345);
        assert_eq!(restored.wins, 3);
        assert_eq!(restored.losses, 1);
        assert_eq!(restored.cooldown_battle, 77);
        assert_eq!(restored.pet_kind.id(), 1);
        assert_eq!(restored.money, 12345);
        assert!(!restored.money_enabled);
        assert!(restored.hard_mode);
    }

    /// A save blob with a different layout/version is rejected (→ fresh egg)
    /// rather than silently misparsed. Regression guard for the pre-money
    /// 96-byte layout (which dropped `drained` and kept SAVE_SIZE at 96)
    /// being reinterpreted with the new field offsets and corrupting the pet.
    #[test]
    #[ignore = "v1 rules; replaced by BornPets v2 plan 1 task 11"]
    fn old_or_wrong_version_save_is_rejected() {
        // A valid current save round-trips.
        let good = GameState::new_egg(7, PetKind::from_u8(2)).to_bytes();
        assert!(GameState::from_bytes(&good).is_some());

        // Old-layout blob one byte short of the versioned size — rejected on length.
        let old = [0xABu8; SAVE_SIZE - 1];
        assert!(GameState::from_bytes(&old).is_none());

        // Right size but wrong version byte — rejected on version.
        let mut wrong_ver = good;
        wrong_ver[0] = SAVE_FORMAT_VERSION.wrapping_add(1);
        assert!(GameState::from_bytes(&wrong_ver).is_none());
    }

    /// A fresh egg starts with the Stage-1 HAX default: 100 balance, money
    /// mode enabled.
    #[test]
    fn new_egg_starts_with_100_hax_and_money_enabled() {
        let state = GameState::new_egg(1, PetKind::DEFAULT);
        assert_eq!(state.money, 100);
        assert!(state.money_enabled);
    }

    /// `add_money` saturates at `u32::MAX` instead of overflowing/panicking,
    /// and every credit flags the state for an immediate save (so the new
    /// balance survives a reboot right after being earned rather than
    /// waiting for the next 15-minute save interval).
    #[test]
    fn add_money_saturates_and_flags_save() {
        let mut state = GameState::new_egg(2, PetKind::DEFAULT);
        state.money = u32::MAX - 5;
        state.add_money(100);
        assert_eq!(state.money, u32::MAX);

        state.mark_saved();
        assert!(!state.needs_save());
        state.add_money(1);
        assert!(state.needs_save(), "add_money should request an immediate save");
    }

    /// Spending succeeds and debits the balance when the pet can afford it.
    #[test]
    fn spend_money_succeeds_when_affordable() {
        let mut state = GameState::new_egg(3, PetKind::DEFAULT);
        state.money = 50;
        assert!(state.spend_money(30));
        assert_eq!(state.money, 20);
    }

    /// Spending fails and leaves the balance unchanged when the pet can't
    /// afford it — money should never go negative (or wrap, in u32 terms).
    #[test]
    fn spend_money_fails_when_too_poor() {
        let mut state = GameState::new_egg(4, PetKind::DEFAULT);
        state.money = 10;
        assert!(!state.spend_money(30));
        assert_eq!(state.money, 10);
    }

    /// `can_afford` reports the exact threshold: affordable at the price,
    /// not affordable one HAX above it.
    #[test]
    fn can_afford_reports_threshold() {
        let mut state = GameState::new_egg(5, PetKind::DEFAULT);
        state.money = 20;
        assert!(state.can_afford(20));
        assert!(!state.can_afford(21));
    }

    /// Every `Action` variant must round-trip through the persisted byte
    /// exactly. Regression test for a real bug: `to_bytes` wrote the
    /// discriminant via a bare `as u8` cast, but `from_bytes`'s
    /// hand-written match only recognized the first four discriminants
    /// (0-3, at the time Feed/Heal/Relax/Play) — Exercise/Medicate/
    /// Ozempic/Drink/Rehab (4-8) all silently came back as `None`,
    /// discarding an in-progress action (and, for Drink specifically,
    /// dropping `active_drink` context too, so the remainder of the
    /// action would have applied under the wrong drink's multipliers
    /// had the match not dropped the action entirely first). `2`
    /// (formerly `Relax`) is now a deliberate gap — see `Action::to_u8`.
    #[test]
    #[ignore = "v1 rules; replaced by BornPets v2 plan 1 task 11"]
    fn every_action_round_trips_through_save() {
        let all = [
            Action::Feed,
            Action::Heal,
            Action::Play,
            Action::Exercise,
            Action::Medicate,
            Action::Ozempic,
            Action::Drink,
            Action::Rehab,
            Action::OnlyPets,
        ];
        for action in all {
            let mut state = GameState::new_egg(5, PetKind::DEFAULT);
            state.active_action = Some(action);
            state.action_ticks_remaining = 3;

            let bytes = state.to_bytes();
            let restored = GameState::from_bytes(&bytes).expect("valid save should parse");
            assert_eq!(
                restored.active_action,
                Some(action),
                "{action:?} did not survive a save/restore round-trip"
            );
            assert_eq!(restored.action_ticks_remaining, 3);
        }

        // No action in progress still round-trips as None, not some
        // stray Some(_).
        let mut state = GameState::new_egg(6, PetKind::DEFAULT);
        state.active_action = None;
        let restored = GameState::from_bytes(&state.to_bytes()).unwrap();
        assert_eq!(restored.active_action, None);
    }

    /// `record_battle` bumps the right counter and always sets the
    /// cooldown, regardless of win/loss.
    #[test]
    fn record_battle_updates_counters_and_cooldown() {
        let mut state = GameState::new_egg(1, PetKind::DEFAULT);
        assert_eq!(state.wins, 0);
        assert_eq!(state.losses, 0);

        state.record_battle(true);
        assert_eq!(state.wins, 1);
        assert_eq!(state.losses, 0);
        assert_eq!(state.cooldown_battle, BATTLE_COOLDOWN());

        state.cooldown_battle = 0;
        state.record_battle(false);
        assert_eq!(state.wins, 1);
        assert_eq!(state.losses, 1);
        assert_eq!(state.cooldown_battle, BATTLE_COOLDOWN());
    }

    /// `friend_boost(true)` used to compute `STAT_MAX() * 2 / 5` with the
    /// multiply in u16 space — since STAT_MAX() is 65535, that overflows
    /// before the divide ever runs (panics under overflow-checks, silently
    /// wraps to roughly half the intended ~40% otherwise). Meeting a brand
    /// new mesh friend must not panic and must actually apply a bigger cut
    /// than the "already-known friend" ~10% boost.
    #[test]
    fn friend_boost_does_not_overflow_and_scales_with_big() {
        let mut state = GameState::new_egg(17, PetKind::DEFAULT);
        state.mood = 0;

        state.friend_boost(true);
        let after_big = state.mood;
        assert!(after_big > 0, "a big friend boost should raise mood");

        state.mood = 0;
        state.friend_boost(false);
        let after_small = state.mood;

        assert!(
            after_big > after_small,
            "meeting a new friend (big={after_big}) should lift mood more than an already-known one (small={after_small})"
        );
    }

    // ── Stage 2: "user can make money" ──────────────────────────────────

    /// Winning a mini-game credits `MINIGAME_HAX_REWARD` HAX when money
    /// mode is on, and grants nothing (but still runs the existing
    /// cooldown/hunger-cost path) when money mode is off.
    #[test]
    fn minigame_win_awards_hax_when_money_enabled() {
        let mut state = GameState::new_egg(30, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money = 0;
        state.money_enabled = true;

        state.award_inspiration(MiniGame::Nim);
        assert_eq!(state.money, MINIGAME_HAX_REWARD);

        state.money = 0;
        state.money_enabled = false;
        state.cooldown_nim = 0;
        state.award_inspiration(MiniGame::Nim);
        assert_eq!(state.money, 0, "money off should grant no HAX");
    }

    /// A mesh Battle win credits `BATTLE_HAX_REWARD` HAX; a loss grants
    /// nothing. With money mode off, even a win grants nothing.
    #[test]
    fn battle_win_awards_hax_winner_only() {
        let mut state = GameState::new_egg(31, PetKind::DEFAULT);
        state.money_enabled = true;
        state.money = 0;

        state.record_battle(true);
        assert_eq!(state.money, BATTLE_HAX_REWARD);

        state.money = 0;
        state.record_battle(false);
        assert_eq!(state.money, 0, "a loss should not award HAX");

        state.money = 0;
        state.money_enabled = false;
        state.record_battle(true);
        assert_eq!(state.money, 0, "money off should grant no HAX even on a win");
    }

    /// `only_pets()` starts the action like `play()`; a second call while
    /// the action is still in progress is rejected.
    #[test]
    fn only_pets_starts_action() {
        let mut state = GameState::new_egg(32, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);

        assert!(state.only_pets());
        assert_eq!(state.active_action, Some(Action::OnlyPets));
        assert!(!state.only_pets(), "action already in progress");
    }

    // ── Stage 3: "things cost money" ────────────────────────────────────

    /// Feeding charges by the food's health tier — healthy (Salad) costs
    /// more than unhealthy (Burger), mirroring `FoodKind::hax_price`.
    #[test]
    fn feed_charges_by_food_health() {
        let mut state = GameState::new_egg(40, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.feed(FoodKind::Salad));
        assert_eq!(state.money, 85, "healthy food (Salad) should cost 15 HAX");

        // Fresh egg so the cooldown from the first feed doesn't reject this one.
        let mut state = GameState::new_egg(41, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.feed(FoodKind::Frikandel));
        assert_eq!(state.money, 90, "unhealthy food (Frikandel) should cost 10 HAX");
    }

    /// Drinking charges by the drink's health tier — healthy (Water) costs
    /// more than unhealthy (Cola), mirroring `DrinkKind::hax_price`.
    #[test]
    fn drink_charges_by_health() {
        let mut state = GameState::new_egg(42, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.drink(DrinkKind::Water));
        assert_eq!(state.money, 85, "healthy drink (Water) should cost 15 HAX");

        let mut state = GameState::new_egg(43, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.drink(DrinkKind::Cola));
        assert_eq!(state.money, 90, "unhealthy drink (Cola) should cost 10 HAX");
    }

    /// The basic Play action costs a flat `PLAY_HAX_COST` (10 HAX).
    #[test]
    fn play_costs_10() {
        let mut state = GameState::new_egg(44, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.play());
        assert_eq!(state.money, 90);
    }

    /// Aspirine (the Heal action) costs a flat `ASPIRINE_HAX_COST` (1 HAX),
    /// and is rejected when the pet can't afford it.
    #[test]
    fn aspirine_costs_1_hax() {
        let mut state = GameState::new_egg(44, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;
        assert!(state.heal());
        assert_eq!(state.money, 99);

        // Broke (0 HAX): heal is rejected, no charge, no action started.
        let mut broke = GameState::new_egg(45, PetKind::DEFAULT);
        broke.update(HATCHING_TICKS() as u32);
        broke.money_enabled = true;
        broke.money = 0;
        assert!(!broke.heal());
        assert_eq!(broke.money, 0);
        assert_eq!(broke.active_action, None);
    }

    /// Each drug action (Ozempic, Medicate, Rehab) costs a flat
    /// `DRUG_HAX_COST` (15 HAX). Medicate/Rehab are gated on
    /// diabetic/alcoholic respectively, so those flags are set first.
    #[test]
    fn drug_costs_15() {
        let mut state = GameState::new_egg(45, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;
        assert!(state.ozempic());
        assert_eq!(state.money, 85, "ozempic should cost 15 HAX");

        let mut state = GameState::new_egg(46, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;
        state.diabetic = true;
        assert!(state.medicate());
        assert_eq!(state.money, 85, "medicate should cost 15 HAX");

        let mut state = GameState::new_egg(47, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;
        state.dependent = true;
        assert!(state.rehab());
        assert_eq!(state.money, 85, "rehab should cost 15 HAX");
    }

    /// With `money_enabled = false`, priced actions proceed but charge
    /// nothing at all — the whole money layer stays inert.
    #[test]
    fn actions_free_when_money_disabled() {
        let mut state = GameState::new_egg(48, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = false;
        state.money = 100;

        assert!(state.feed(FoodKind::Salad));
        assert_eq!(state.money, 100, "feed should not charge when money is off");

        let mut state = GameState::new_egg(49, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = false;
        state.money = 100;

        assert!(state.play());
        assert_eq!(state.money, 100, "play should not charge when money is off");
    }

    /// Stage 5 reverses the Stage-3 best-effort contract: an unaffordable
    /// action is now REJECTED outright rather than proceeding for free —
    /// `feed()` returns false, the balance is untouched, and no action starts.
    #[test]
    fn broke_action_is_now_rejected() {
        let mut state = GameState::new_egg(50, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 5;

        assert!(!state.feed(FoodKind::Salad), "unaffordable action should be rejected");
        assert_eq!(state.money, 5, "a rejected charge should not touch the balance");
        assert_eq!(state.active_action, None);
    }

    /// A rejected action (already busy) must never charge — the charge
    /// line sits after the guard checks, so a second `feed()` call while
    /// one is in progress returns false and leaves `money` unchanged.
    #[test]
    fn rejected_action_does_not_charge() {
        let mut state = GameState::new_egg(51, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.feed(FoodKind::Salad));
        state.money = 100; // reset after the first (accepted) charge

        assert!(!state.feed(FoodKind::Frikandel), "feed should be rejected while busy");
        assert_eq!(state.money, 100, "a rejected action must not charge");
    }

    // ── Stage 5: "broke" branch + affordability gating ─────────────────

    /// An unaffordable priced action (Salad costs 15, balance is 5) is
    /// rejected: `feed()` returns false, the balance is untouched, and no
    /// action starts.
    #[test]
    fn unaffordable_action_is_rejected() {
        let mut state = GameState::new_egg(60, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 5;

        assert!(!state.feed(FoodKind::Salad));
        assert_eq!(state.money, 5);
        assert_eq!(state.active_action, None);
    }

    /// An affordable priced action charges its price and starts normally.
    #[test]
    fn affordable_action_charges_and_starts() {
        let mut state = GameState::new_egg(61, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 100;

        assert!(state.feed(FoodKind::Salad));
        assert_eq!(state.money, 85);
        assert_eq!(state.active_action, Some(Action::Feed));
    }

    /// With `money_enabled = false`, the affordability gate never applies —
    /// the action proceeds free even at 0 balance.
    #[test]
    fn money_disabled_ignores_affordability() {
        let mut state = GameState::new_egg(62, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = false;
        state.money = 0;

        assert!(state.feed(FoodKind::Salad));
        assert_eq!(state.money, 0);
    }

    /// Only-pets is the escape hatch when broke and must never be gated on
    /// affordability — it starts even at `money == 0`.
    #[test]
    fn only_pets_never_gated_when_broke() {
        let mut state = GameState::new_egg(63, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.money = 0;

        assert!(state.only_pets());
    }

    // ── Hard (US) mode: pricier healthy food / medication / rehab ──────

    /// In Hard mode, healthy food (Salad) costs 20 HAX instead of the
    /// normal 15. Normal (non-hard) mode is unaffected — regression check.
    #[test]
    fn hard_mode_healthy_food_costs_20() {
        let mut state = GameState::new_egg(70, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.hard_mode = true;
        state.money = 100;

        assert!(state.feed(FoodKind::Salad));
        assert_eq!(state.money, 80, "hard-mode healthy food should cost 20 HAX");

        let mut normal = GameState::new_egg(71, PetKind::DEFAULT);
        normal.update(HATCHING_TICKS() as u32);
        normal.money_enabled = true;
        normal.hard_mode = false;
        normal.money = 100;

        assert!(normal.feed(FoodKind::Salad));
        assert_eq!(normal.money, 85, "normal-mode healthy food should still cost 15 HAX");
    }

    /// In Hard mode, medication (Ozempic / Insulin) costs 45 HAX — 3x the
    /// normal 15. Normal mode is unaffected.
    #[test]
    fn hard_mode_medication_costs_45() {
        let mut state = GameState::new_egg(72, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.hard_mode = true;
        state.money = 100;

        assert!(state.ozempic());
        assert_eq!(state.money, 55, "hard-mode ozempic should cost 45 HAX");

        let mut normal = GameState::new_egg(73, PetKind::DEFAULT);
        normal.update(HATCHING_TICKS() as u32);
        normal.money_enabled = true;
        normal.hard_mode = false;
        normal.money = 100;

        assert!(normal.ozempic());
        assert_eq!(normal.money, 85, "normal-mode ozempic should still cost 15 HAX");
    }

    /// In Hard mode, Rehab costs 75 HAX — 5x the normal 15. Normal mode
    /// is unaffected.
    #[test]
    fn hard_mode_rehab_costs_75() {
        let mut state = GameState::new_egg(74, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.hard_mode = true;
        state.dependent = true;
        state.money = 100;

        assert!(state.rehab());
        assert_eq!(state.money, 25, "hard-mode rehab should cost 75 HAX");

        let mut normal = GameState::new_egg(75, PetKind::DEFAULT);
        normal.update(HATCHING_TICKS() as u32);
        normal.money_enabled = true;
        normal.hard_mode = false;
        normal.dependent = true;
        normal.money = 100;

        assert!(normal.rehab());
        assert_eq!(normal.money, 85, "normal-mode rehab should still cost 15 HAX");
    }

    /// Hard mode only raises healthy food / medication / rehab — unhealthy
    /// food and the flat Play cost are untouched.
    #[test]
    fn hard_mode_unhealthy_food_and_play_unchanged() {
        let mut state = GameState::new_egg(76, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.hard_mode = true;
        state.money = 100;

        assert!(state.feed(FoodKind::Frikandel));
        assert_eq!(state.money, 90, "hard mode should not touch unhealthy food price");

        let mut state = GameState::new_egg(77, PetKind::DEFAULT);
        state.update(HATCHING_TICKS() as u32);
        state.money_enabled = true;
        state.hard_mode = true;
        state.money = 100;

        assert!(state.play());
        assert_eq!(state.money, 90, "hard mode should not touch the flat Play price");
    }
}
