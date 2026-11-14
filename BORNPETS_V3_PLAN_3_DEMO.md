# BornPets v3, plan 3: demo mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a pet that looks after itself on the desk. It eats, drinks, sleeps, washes, plays and takes its medicine on its own. Its condition wanders, but it never reaches Leaving. It sleeps when the player sleeps. The player can step in at any time, and the pet thanks them. The screen stays calm: a short note ("had an apple") instead of toasts, no buzzer, no red unless the pet really could use a hand. Other plans can read one flag (`is_demo`) so demo pets battle only demo pets (plan 5).

**Architecture:** the policy lives in one new engine module, `src/game/engine/demo.rs`. It does not poll and it adds no fixed-step loop. Every policy threshold is a solver line (`solver::extra_levels`), and every policy delay is an engine timer (`solver::timer_bound`), so a segment never steps over a choice. At each segment boundary `update` calls `demo::run`, which settles the bookkeeping (`settle`), asks the pure `decide` what to do, and acts by calling the same public starters the menu uses (`feed`, `drink`, `sleep`, ...). The pet's own doses land at once: the starter is called and the action is finished in the same minute, so no ticks are consumed and split invariance holds. `next_wake_tick` sees a coming choice because `decide(..).is_some()` is part of the `Visible` tuple and the look-ahead clone is settled the same way `update` settles the real pet. Demo state is saved in 11 of the 28 reserved bytes of the format-3 save; the format stays version 3, 177 bytes.

**Tech stack:** Rust, `no_std` firmware (embassy, embedded-graphics, FONT_7X13 / FONT_7X13_BOLD only), host tests through the `simulator` feature.

**Spec:** `BORNPETS_V3_DESIGN.md` (repo root): 8.1 (policy table), 8.2 (floor), 8.3 (calm display), 8.4 (player help), 8.5 (toggle), 13.3 (policy as solver events), 15.4 (dependence never set in demo), 17 rows 11 to 15, 18.3 (demo results), 18.8 (fitness floor), 18.9 ("Tune it in plan 3"). Prototype policy: `prototypes/bornpets-simv3/src/main.rs` `demo_policy` (about lines 451 to 505). Roadmap row 3: `BORNPETS_V3_ROADMAP.md`. Rulings R1 to R26 and plan 2 rulings: `.superpowers/sdd/BORNPETS_V3_PLAN_1_ENGINE/global-constraints.md`, `BORNPETS_V3_PLAN_2_UI.md` "Self-review". Format exemplar: `BORNPETS_V3_PLAN_2_UI.md`.

**Code base:** worktree `BornHack-Cyberegg-v3demo`, branch `ranzbak/bornpets-v3-demo`, HEAD `64027d4`. Anchors at that commit:
- `src/game/engine/mod.rs`: `GameState` 239 (`money_enabled` 328), `new_egg` 398, `update` 532, `segment` ~600, `consume_action_ticks` ~690, `start_action` 700, starters `feed` 729 to `rehab` 826, `preview` 843, `sleep` 935, `wake` 951, `award_inspiration` 983, `Visible` 1011, `refusal_for` 1024, `visible` 1155, `next_wake_tick` 1184, `PetStats` ~1240 (`money_enabled` 1274), `stats` 1397, `record_battle` 1555, `add_money` 1572, `SAVE_SIZE` 1611, `to_bytes` 1738 (last field `saved_unix` ~1828), `from_bytes` 1834.
- `src/game/engine/advance.rs`: `advance` 55, fitness step 123, dependence onset 143, leaving 223 to 236, care counters 242, `wake` 250 (groggy fitness 292).
- `src/game/engine/solver.rs`: `extra_levels`, `level_bound`, `timer_bound`, `seg_len` 272, `sig` 301 (health lines 25 and 45, "no term reads them yet").
- `src/game/engine/doses.rs`: `Refusal` 8, `Reason` 18, `Refusal::text` 44, `Reason::pack` 78, `minigame` 262, `fall_asleep` 392, `wake_up` 405.
- `src/game/engine/status.rs`: `Word` 133, `status_line`. `causes.rs`: `Cause` 82, `PLAIN` 104, `what` 124, `why` 147, `is_warning` 170, `pack`. `hint.rs`: `Subject` 37, `hint` 215. `thresholds.rs`: `thresholds_table!` 63, `CLASSIC` ~430, `CASUAL` ~700, `BORNPETS_CFG_KEYS` 989, `clamp_*` 1592 to 1611, `validate`, `cfg_template`. `sim.rs`: `Driver` 27, `Outcome` 60, `install_preset` 138, `run_routine` 155, `run_full` 242, `note_segment`.
- `src/game/lifecycle.rs`: `is_started` 185, `on_wall_clock_set` 289, `cycle` 387, `log_causes` 400, `status_line` ~455, `check_severity_transition` ~530, onset alerts 580 to 610, `money_enabled` 982.
- `src/game/mod.rs`: `status_wants_full_refresh` 427, `show_cause` 482, `HAX_POS` 602, `draw_status_line` 634, HAX draw 808 to 822.
- `src/menu.rs`: `BORNAGOTCHI_ITEMS` 1977 (7 items).
- `src/lib.rs`: `tz_offset_secs` 194, `unix_now` 484. `src/platform.rs`: `xorshift32` 59.
- `tests/engine_v3.rs`: `split_invariance` 659, tick remainder tests 1666 to 1715, `status_wake` 2147, `oracle` 3053, template tests 3261 to 3320, `causes` 4027, `hint` 4663. `.github/workflows/tests.yml` (matrix line 28, `others` loop 43 to 47).

## Global Constraints

Every task's requirements include this section.

**Owner rules (verbatim where quoted):**
- "the delta engine as it's written now needs to stay the algorithm to drive progress". No fixed-step loop and no periodic polling. Every demo choice is an engine event with a computable boundary: a level line in `solver::extra_levels` or a timer in `solver::timer_bound`. A step loop may exist only as a simulator test oracle (`Driver::Oracle`).
- Demo mode is a desk companion. The pet eats, sleeps and takes its medication itself. Its condition wanders but never reaches Leaving. The player can step in. The policy covers play, boredom, hygiene and treats. The pet never exercises by itself (prototype row 12): exercise stays the player's part. "Nice to watch, not distracting."
- Demo pets battle only demo pets. That rule is plan 5; this plan only provides the flag (`lifecycle::is_demo`, `PetStats::demo`).
- Fitness floor about 40 % (owner). Classic uses 45 because the Classic fitness band starts at 43 (`FIT_LO`), Casual uses 40; see "Needs the owner" (a).
- Sleep follows the player's rhythm. There is no fixed sleep timer. The wall clock (`unix_now` plus the time zone) may be unset after boot; the pet then runs on its own rhythm (energy-driven) until the clock is set.
- Every disabled or refused action shows its reason, from the one mapping (R24: `doses::Reason::text`). A hint never suggests an action the menu would refuse. The demo pet never takes a choice its starter would refuse; it moves on to the next one.
- "If the labels in the game need to be changed to make the game flow more clear to the player, do so." Culture references stay (`Frikandel spec`).
- Every balance value is a `Thresholds` field with a Classic and a Casual value and a `BPETS_V3.CFG` key. The template `assets/to-badge/BPETS_V3.CFG` is regenerated in the task that adds a key; its rows read `# KEY=N   # Casual M`. Display pacing stays const.
- "Never write the old kv key `state`, `game/name` or `BORNPETS.CFG`. The save is `state_v3`." Demo state goes into the reserved tail of the format-3 save (bytes 149 to 159); version byte 3 and `SAVE_SIZE` 177 stay. A plan-2 save has zeros there, which reads as "demo off". Bytes 160 to 176 (17 bytes) stay reserved for plan 6.
- DRY: the pet's choices call the real public starters (`feed`, `drink`, `wash`, `relax`, `play`, `heal`, `medicate`, `rehab`, `sleep`, `wake`, `award_inspiration`), which call `start_action` and the `doses::*` functions. Nothing re-implements a dose, a refusal or a price.
- Flash budget: the SSD1680 release image is about 853 KiB of the 960 KiB gate. If space runs short, the owner allows removing the Lights Out mini-game. This plan adds no font and no sprite.
- Local commands use at most 2 cores, one cargo command at a time: `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo ... -j 2` (also for `cargo sim`). Never run `cargo fmt` on the whole tree. Never use `git stash`.
- **Test policy (owner, 2026-11-14):** each task runs **only its own focused tests** locally. No task pushes to CI. The full suite, both firmware builds, the flash gate and the long demo balance runs happen once, in Task 8 (regression), on the GitHub mirror. Task 9 fixes what Task 8 finds.
- Push only to the `github` remote (public mirror `ranzbak/bornhack-firmware-2026-ce`), never to codeberg, and only in Tasks 8 and 9. SSH git commands are prefixed `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh`.
- Commit footer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Carried from plans 1 and 2:** integer math only in `src/game/engine/` (except `sim.rs`); no `alloc` in engine code; levels are `u16` with `P = 655` per percent; 1 engine minute = 6 ticks; engine wake cap `MAX_SLEEP_TICKS` 720 ticks; R15 (code this plan makes dead is removed in the same commit, no commented-out code); R24; R26 (call it "BornPets v3"); R7 and R13 (the known pre-existing failures `name_screen::tests::drops_control_and_beyond_latin1` and the `signed_channel.rs` doctest are not yours). Buzz only for danger or the alerting status words, and in demo mode not at all.

**Width budgets** (7 px per column; both panels 152 x 152):

| Where | Budget |
|---|---|
| Toast (top-left, shares the HAX / DEMO row) | `what` ≤ 12, `why` ≤ 15 columns |
| `DEMO` tag (right-aligned at `HAX_POS`, x = 150) | 4 columns |
| Status line, demo note, help line (bottom of the pet area, centred) | ≤ 21 columns per line, at most 2 lines |
| Hint popover body | ≤ 18 columns |
| Game settings menu row (`MENU_W` 144, label only, no stepper) | ≤ 20 columns |
| Reason text (`Reason::text`, `heapless::String<16>`) | ≤ 16 columns |

Each text table gets a worst-case width test in the task that adds it.

**Verify (V)**, the last step of Tasks 1 to 7:
1. Run the task's focused tests (the commands are in the task). Nothing else.
2. `git status --short` shows only the files the task names.
3. No push. Report the test names run and their result.

## Review Focus

The failure modes most likely to reach a player unnoticed, most likely first. Each is pinned by a test.

1. **The pet acts but the screen does not show it** (or the badge wakes for nothing). A choice fires at a level line or timer the solver does not bound, or the look-ahead clone is not settled the way `update` settles the pet, so `next_wake_tick` sleeps past the choice for up to 2 h. Pinned in Task 3 by `demo_wake_is_exact_on_random_states` (2000 seeded demo states: `next_wake_tick` equals the first minute the `Visible` tuple changes) and in Task 4 by `the_badge_wakes_when_the_pet_acts`.
2. **The pet's own doses make the engine depend on how time was split.** Pinned in Task 4 by `demo_update_is_split_invariant` (one long update equals the same span cut at random ticks, with acts in between) and in Task 8 by the oracle suite on demo routines (`demo_delta_matches_oracle`).
3. **The demo pet reaches Leaving, or a floor hands out free stats.** The fitness and health floors must stop the fall, not raise a stat that was already below. Pinned in Task 2 by `fitness_floor_stops_the_fall_without_raising`, `health_floor_stops_damage`, `demo_pet_never_leaves`, and in Task 8 by the 30-day runs.
4. **The save breaks.** A plan-2 save must load with demo off, the demo block must round-trip, the plan-6 tail must stay zero, and no old key is written. Pinned in Task 1 by `plan2_save_loads_with_demo_off`, `demo_block_round_trips`, `tail_after_demo_block_stays_zero`, `demo_never_writes_old_keys`.
5. **Words contradict each other.** The pet takes a refused choice, the hint asks for a refused action, the pet's own warnings toast every hour, or a note survives a status change. Pinned by `demo_never_takes_a_refused_choice` (Task 3), `demo_hint_is_never_refused` (Task 6), `pet_causes_do_not_toast` and `a_note_ends_when_the_line_changes` (Task 5).

## File map

| File | Change |
|---|---|
| `src/game/engine/demo.rs` | create: `Demo` (saved block), `Choice`, `Note`, `settle`, `decide`, `run`, `levels`, `timers`, `wake_view`, `floor` |
| `src/game/engine/mod.rs` | `demo: Demo` and transient fields; `set_demo`, `is_demo`, `money_live`, `sync_clock`, `self_raised`; `update` calls `demo::run`; `start_action` arms the player pause; `finish_action` split out of `consume_action_ticks`; `Visible` gains `demo::wake_view`; `next_wake_tick` settles the clone; save bytes 149 to 159; `PetStats::{demo, self_raised}` |
| `src/game/engine/advance.rs` | demo timers count down; fitness and health floors; no leaving, no dependence onset, no care counting in demo; day minute advances |
| `src/game/engine/solver.rs` | `extra_levels` chains `demo::levels`; `timer_bound` reads `demo::timers`; `sig` health lines read the demo accessors |
| `src/game/engine/doses.rs` | `Refusal::FixMeFirst` |
| `src/game/engine/causes.rs` | `Cause::Thanks` |
| `src/game/engine/status.rs` | `status_line` never sets `danger` in demo |
| `src/game/engine/hint.rs` | `Subject::Stiff` |
| `src/game/engine/thresholds.rs` | 52 `DEMO_*` fields, Classic and Casual values, keys, `validate` rules |
| `assets/to-badge/BPETS_V3.CFG` | regenerated |
| `src/game/engine/sim.rs` | `Setup`, `run_with`, demo `Outcome` fields, routines `Nobody` and `Helper` |
| `src/game/lifecycle.rs` | `local_minute`, `sync_clock` calls, `is_demo`, `toggle_demo`, `demo_menu_label`, `pet_line`; calm rules (no buzzer, toast rule); `money_enabled` reads `money_live` |
| `src/game/mod.rs` | `PetLine` drawing, `DEMO` tag, full refresh follows `pet_line` |
| `src/menu.rs` | `BORNAGOTCHI_ITEMS` gains the demo row (8 items) |
| `tests/engine_v3_demo.rs` | create: all demo engine tests, long runs `#[ignore]` |
| `tests/engine_v3.rs` | template test regeneration hook only |
| `tests/engine_v3_casual.rs` | one Casual demo run |
| `.github/workflows/tests.yml` | matrix suite `engine_v3_demo` with `--include-ignored`; excluded from `others` |
| `GAME.md`, `USER_GAMES.md` | demo mode section, settings row, key list |

---

### Task 1: demo state, save block, toggle, frozen money, tunables

Spec 8.5: "one save byte", HAX frozen, guard "fix me first". This task adds the state and every `DEMO_*` tunable so later tasks only read them.

**Files:**
- Create: `src/game/engine/demo.rs` (struct and save helpers only), `tests/engine_v3_demo.rs`.
- Modify: `src/game/engine/mod.rs` (`pub mod demo;`, fields, `set_demo`, `money_live`, `to_bytes`, `from_bytes`, `new_egg`, `start_action` price filter, `award_inspiration`, `record_battle`, `add_money`, `stats`), `src/game/engine/doses.rs` (`Refusal::FixMeFirst`, its `text` and `pack`/`unpack` code), `src/game/engine/thresholds.rs`, `assets/to-badge/BPETS_V3.CFG`, `tests/engine_v3.rs` (regeneration hook), `USER_GAMES.md` ("Custom balance" key list).

**Interfaces:**
- Consumes: nothing new.
- Produces:
```rust
// demo.rs
/// The saved demo block (save bytes 149..160). All zero = demo off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Demo {
    pub on: bool,
    pub soft_armed: bool,   // soft delay `next_min` is running
    pub clock_armed: bool,  // `clock_min` counts to bedtime (awake) or getting up (asleep)
    pub lazy: u8,           // temperament 0..=DEMO_LAZY_MAX_PCT, re-rolled every DEMO_LAZY_MIN
    pub clock_min: u16,
    pub whim_min: u16,
    pub lazy_min: u16,
    pub next_min: u8,       // soft delay or dry-out interval
    pub pause_min: u8,      // policy paused after the player acted
    pub demo_min: u32,      // minutes lived in demo; saved as whole hours (u16)
}
pub const BLOCK_AT: usize = 149;
pub const BLOCK_LEN: usize = 11;
impl Demo {
    pub fn write(&self, b: &mut [u8; BLOCK_LEN]);  // flags u8 (bit0 on, bit1 soft_armed, bit2 clock_armed, bits4..7 lazy), clock u16, whim u16, lazy u16, next u8, pause u8, hours u16; little-endian like the rest
    pub fn read(b: &[u8; BLOCK_LEN]) -> Demo;      // demo_min = hours * 60
}
// mod.rs, GameState
pub demo: demo::Demo,
pub(crate) demo_rng: u32,          // transient: new_egg seeds it from `seed`; from_bytes derives it from age_ticks and generation (never 0)
pub fn set_demo(&mut self, on: bool) -> Result<(), doses::Reason>;
pub fn is_demo(&self) -> bool;     // self.demo.on
pub fn money_live(&self) -> bool;  // self.money_enabled && !self.demo.on
pub fn self_raised(&self) -> bool; // demo_min * 4 > age in minutes (spec 17 row 11; plan 8 records it)
// PetStats
pub demo: bool,
pub self_raised: bool,
// PetStats::money_enabled becomes self.money_live()
// doses.rs
Refusal::FixMeFirst  // text "fix me first"
```
- `set_demo(true)`: `Err(Reason::Egg | Gone | Hibernating)` as `cannot_act` reports them (reuse it, but not `Busy`: a running action does not block the toggle); `Err(Reason::Refused(Refusal::FixMeFirst))` when `phase == Leaving` or health < `DEMO_ON_HEALTH_PCT`. On success: `demo.on = true`, `lazy_min = 0` (so the first `settle` rolls the temperament), `whim_min = DEMO_WHIM_BASE_MIN`, everything else zero, `demo_min` kept, `request_save()`. `set_demo(false)` always succeeds: keeps only `demo_min`, `request_save()`.
- `money_live()` replaces `money_enabled` in `start_action` (price filter), `award_inspiration`, `record_battle`, the work reward in `doses.rs` (~385), and `PetStats::money_enabled`. `add_money` returns early in demo. The balance itself is untouched, so turning demo off gives the same HAX back.

**Tunables** (all added in this task; Classic / Casual; `_PCT` fields are `u8` with `clamp_pct`, `_MIN` fields `u16` with `clamp_u16` unless noted):

| Key | Classic | Casual | Meaning |
|---|---|---|---|
| `DEMO_FIT_FLOOR_PCT` | 45 | 40 | fitness stops falling here |
| `DEMO_HEALTH_FLOOR_PCT` | 25 | 25 | health stops falling here |
| `DEMO_HELP_HEALTH_PCT` | 35 | 35 | below: red `could use a hand` |
| `DEMO_ON_HEALTH_PCT` | 40 | 40 | below: demo cannot be turned on |
| `DEMO_MED_HEALTH_PCT` | 45 | 45 | below: the pet takes medicine |
| `DEMO_WAKE_FULL_PCT` | 18 | 18 | asleep and fullness below: wakes to eat |
| `DEMO_EDGE_FULL_PCT` | 20 | 20 | edge: eats at once |
| `DEMO_EDGE_WATER_PCT` | 22 | 22 | edge: drinks water at once |
| `DEMO_EDGE_ENERGY_PCT` | 12 | 12 | edge: sleeps at once |
| `DEMO_EDGE_BOOZE_PCT` | 55 | 55 | above: only water, once per dry-out interval |
| `DEMO_SOFT_WATER_PCT` | 48 | 43 | hydration + lazy below: drinks |
| `DEMO_SOFT_FULL_PCT` | 36 | 31 | fullness + lazy below: eats |
| `DEMO_SOFT_ENERGY_PCT` | 27 | 22 | energy + lazy/2 below: goes to bed (own rhythm only) |
| `DEMO_SOFT_DIRT_PCT` | 60 | 65 | dirt − lazy above: washes |
| `DEMO_SOFT_FUN_PCT` | 27 | 22 | fun + lazy below: plays |
| `DEMO_PLAY_ENERGY_PCT` | 25 | 25 | plays only above this energy |
| `DEMO_WIRED_FUN_PCT` | 78 | 83 | fun − lazy above: relaxes |
| `DEMO_RISE_ENERGY_PCT` | 97 | 97 | own rhythm: getting up is armed here |
| `DEMO_NAP_ENERGY_PCT` | 55 | 55 | whim: energy below and ... |
| `DEMO_NAP_FUN_PCT` | 45 | 45 | ... fun below: naps |
| `DEMO_WHIM_BOOZE_LO_PCT` | 15 | 15 | whim: booze below: beer or cake |
| `DEMO_WHIM_BOOZE_HI_PCT` | 45 | 45 | whim: booze below: maybe a beer |
| `DEMO_WHIM_BEER_LO_PCT` | 50 | 50 | chance of beer (else cake) under the low line |
| `DEMO_WHIM_BEER_HI_PCT` | 35 | 35 | chance of beer under the high line |
| `DEMO_BEER_STOP_MIN` | 300 | 300 | dependence counter above this: no whim beer |
| `DEMO_LIGHT_FOOD_PCT` | 56 | 56 | weight above: junk becomes salad, no cake (tuned in Task 9) |
| `DEMO_RICH_FOOD_PCT` | 42 | 42 | weight below: healthy becomes pizza |
| `DEMO_REACT_BASE_MIN` / `_SPREAD_MIN` | 10 / 20 | same | soft delay |
| `DEMO_DRY_BASE_MIN` / `_SPREAD_MIN` | 20 / 40 | same | dry-out water interval |
| `DEMO_WHIM_BASE_MIN` / `_SPREAD_MIN` | 180 / 600 | same | whim timer |
| `DEMO_LAZY_MIN` | 360 | 360 | temperament re-roll |
| `DEMO_LAZY_MAX_PCT` | 15 | 15 | temperament range (clamped to 15: it is 4 bits) |
| `DEMO_NAP_BASE_MIN` / `_SPREAD_MIN` | 40 / 60 | same | whim nap |
| `DEMO_EDGE_NAP_MIN` | 120 | 120 | daytime nap after the edge sleep |
| `DEMO_RISE_BASE_MIN` / `_SPREAD_MIN` | 10 / 110 | same | own rhythm: delay before getting up |
| `DEMO_BED_AT_MIN` / `DEMO_BED_SPREAD_MIN` | 1320 / 90 | same | bedtime 22:00 to 23:30 (clamped to 1439 / 600) |
| `DEMO_UP_AT_MIN` / `DEMO_UP_SPREAD_MIN` | 420 / 90 | same | getting up 07:00 to 08:30 |
| `DEMO_PAUSE_MIN` | 30 | 30 | policy pause after the player acts (`u8`, `v.min(255)`) |
| `DEMO_PICK_APPLE_PCT` / `SALAD` / `PIZZA` / `FRIK` | 40 / 15 / 25 / 12 | same | food draw; cake is the rest |
| `DEMO_PICK_WATER_PCT` / `COLA` | 70 / 20 | same | drink draw; beer is the rest |
| `DEMO_PICK_GAME_PCT` | 30 | 30 | chance a play is a self-played mini-game |

`validate` adds: the four food picks summing over 100, or the two drink picks over 100, fall back to the preset values; `DEMO_HEALTH_FLOOR_PCT < DEMO_HELP_HEALTH_PCT < DEMO_ON_HEALTH_PCT` and `DEMO_HELP_HEALTH_PCT <= DEMO_MED_HEALTH_PCT` else the preset four; `DEMO_RICH_FOOD_PCT < DEMO_LIGHT_FOOD_PCT` else both preset. The Casual soft lines are the Classic ones moved 5 points toward the wider Casual bands (planner's call; Task 9 may retune).

- [ ] **Step 1: write the failing tests** in `tests/engine_v3_demo.rs`, `mod state`:
```rust
#[test] fn plan2_save_loads_with_demo_off() {
    // a plan-2 save: new_egg(7, PetKind(0)) made Active, to_bytes, bytes 149..177 zeroed
    // from_bytes -> Some(s); !s.demo.on; s.demo == Demo::default()
}
#[test] fn demo_block_round_trips() {
    // demo on, soft_armed, clock_armed, lazy 11, clock 777, whim 650, lazy_min 300, next 29, pause 30, demo_min 3 * 60 + 59
    // from_bytes(to_bytes) .demo == same with demo_min 180 (whole hours)
}
#[test] fn tail_after_demo_block_stays_zero() { /* every field maxed: to_bytes()[160..177] all 0; b[0] == 3; len 177 */ }
#[test] fn demo_never_writes_old_keys() {
    // SAVE_KEY == "state_v3"; demo.rs source has no "\"state\"", "BORNPETS.CFG", "game/name"
    // (include_str!("../src/game/engine/demo.rs"))
}
#[test] fn demo_turns_on_only_for_a_pet_in_fair_shape() {
    // egg -> Err(Reason::Egg); health 39 % -> Err(Refused(FixMeFirst)); Leaving -> same;
    // health 60 % -> Ok, is_demo(); set_demo(false) -> Ok, !is_demo(), demo_min kept
}
#[test] fn demo_freezes_hax() {
    // money_enabled, money 50, demo on: feed(Pizza) leaves money 50; award_inspiration adds 0;
    // record_battle(true) adds 0; stats().money_enabled == false; demo off: feed(Pizza) charges
}
#[test] fn self_raised_needs_a_quarter_of_the_life() { /* age 400 min, demo_min 100 -> false; 101 -> true */ }
#[test] fn fix_me_first_fits() { /* Reason::Refused(FixMeFirst).text().len() <= 16; pack/unpack round trip */ }
```
In `tests/engine_v3.rs`, `template_parses_to_classic_with_no_edits`: before the first assert, `if std::env::var_os("BPETS_REGEN").is_some() { std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/to-badge/BPETS_V3.CFG"), &template).unwrap(); }` (planner's call: one line that makes the regeneration reproducible).

- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo state::` fails (`demo` missing).

- [ ] **Step 3: implement.** `Demo`, the fields, `set_demo`, `money_live`, `self_raised`, the save block written right after `saved_unix` (assert in `to_bytes` with `debug_assert_eq!(w.pos(), demo::BLOCK_AT)` or the writer's equivalent), the tunables, the setters, `validate`. Regenerate the template: `BPETS_REGEN=1 CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3 template_`. Regenerate the `USER_GAMES.md` key list from `BORNPETS_CFG_KEYS` (one line per new key, Classic and Casual).

- [ ] **Step 4: run them and see them pass.** `... --test engine_v3_demo state::`, `... --test engine_v3 template_`, `... --test save_format`: PASS.
- [ ] **Step 5: commit** `feat(bornpets): demo mode state, save block and tunables`.
- [ ] **Step 6: V.**

### Task 2: the floor (spec 8.2)

The demo pet's condition wanders but cannot fall through: fitness and health stop at their floors, the pet never starts leaving, it never becomes dependent, and its time does not count toward the care score.

**Files:**
- Modify: `src/game/engine/advance.rs`, `src/game/engine/solver.rs` (`extra_levels`, `sig`), `src/game/engine/demo.rs` (`floor`, `levels`).
- Test: `tests/engine_v3_demo.rs` `mod floors`.

**Interfaces:**
- Consumes: Task 1 `Demo`, `DEMO_FIT_FLOOR_PCT`, `DEMO_HEALTH_FLOOR_PCT`.
- Produces:
```rust
// demo.rs
/// A fall that crosses `floor` stops on it; a stat already below it does not fall further and is not raised.
pub fn floor(before: u16, after: u16, floor: u16) -> u16; // after.max(before.min(floor))
/// Demo lines for one need (levels, as `lvl(..)`), appended by solver::extra_levels while demo is on
/// (Task 3 adds the policy lines).
pub fn levels(s: &GameState, need: Need) -> ([i32; 8], usize);
```
- `advance`, demo on: the fitness step (123) and the groggy fitness drop (`wake` 292) go through `demo::floor(.., lvl(DEMO_FIT_FLOOR_PCT()))`; health damage goes through `demo::floor(.., lvl(DEMO_HEALTH_FLOOR_PCT()))`; the leaving block (223) is skipped (leaving_min decays as when healthy); the dependence onset (143) is skipped; `care_good_min` / `care_awake_min` (242) are not counted; `demo_min += dt`.
- `solver::extra_levels` (32) appends `demo::levels(s, need)` while demo is on; `MAX_EXTRA` (30) rises from 8 to 16, since Energy gets 5 demo lines in Task 3. The fitness floor is a Fit line; the health lines are in `sig`, so a segment stops on both. `sig`: the literal health lines 25 and 45 become `DEMO_HEALTH_FLOOR_PCT()` and `DEMO_MED_HEALTH_PCT()`, and `DEMO_HELP_HEALTH_PCT()` is added.

- [ ] **Step 1: write the failing tests:**
```rust
#[test] fn fitness_floor_stops_the_fall_without_raising() {
    // demo, fitness 50 %, lazy pet (fitness falls): advance 10 days -> fitness == lvl(45)
    // demo, fitness 30 %: advance 1 day -> fitness == 30 % (not raised, not lower)
}
#[test] fn health_floor_stops_damage() { /* demo, fullness 0, hydration 0, health 30 %: advance 3 days -> health == lvl(25) */ }
#[test] fn demo_pet_never_leaves() { /* same starving pet: phase stays Active, leaving_min == 0 after 3 days; demo off: Leaving */ }
#[test] fn demo_never_becomes_dependent() { /* booze 80 % held by repeated beers over 2 days: !dependent */ }
#[test] fn demo_time_is_not_care_time() { /* care_awake_min unchanged over 600 min; demo_min += 600 */ }
#[test] fn floors_are_segment_lines() { /* demo, fitness 46 % falling: solver::seg_len stops at the minute fitness reaches 45 */ }
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo floors::`.
- [ ] **Step 3: implement** `floor`, `levels` (floors only), the `advance` rules, the `sig` accessors.
- [ ] **Step 4: run them and see them pass.** `... --test engine_v3_demo floors::` and `... --test engine_v3 split_invariance::`: PASS (the second shows the non-demo engine is unchanged).
- [ ] **Step 5: commit** `feat(bornpets): demo floor keeps the pet from leaving`.
- [ ] **Step 6: V.**

### Task 3: the clock, the timers and the choice (no acting yet)

Spec 8.1 and 13.3: "policy thresholds are solver boundaries"; "an action is itself an event". This task makes the choice visible to the wake but does not act on it yet.

**Files:**
- Modify: `src/game/engine/demo.rs`, `src/game/engine/mod.rs` (`sync_clock`, `day_min`, `Visible`, `visible`, `next_wake_tick`), `src/game/engine/advance.rs` (timers, `day_min`), `src/game/engine/solver.rs` (`timer_bound`), `src/game/engine/doses.rs` (`fall_asleep` and `advance::wake` disarm the clock), `src/game/lifecycle.rs` (`local_minute`, `sync_clock` calls).
- Test: `tests/engine_v3_demo.rs` `mod policy`, `mod wake`.

**Interfaces:**
- Consumes: Task 1, Task 2 `levels`.
- Produces:
```rust
// mod.rs, GameState (transient, not saved)
pub(crate) day_min: Option<u16>,   // local minute of day at last_update_tick; None = own rhythm
/// Tell the engine the local time `local_min` (0..1440) at `now_tick`. Stores the minute at last_update_tick.
pub fn sync_clock(&mut self, now_tick: u32, local_min: Option<u16>);
// demo.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    Eat(FoodKind), Drink(DrinkKind), Sleep, Nap, Wake,
    Wash, Play, Game(MiniGame, u8), Relax, Medicine, Insulin, Rehab,
}
/// Bookkeeping at a segment boundary: re-roll the temperament, arm or disarm the soft delay,
/// arm the clock, re-arm a whim that has nothing to do. Never acts, never changes `visible()`.
pub fn settle(s: &mut GameState);
/// What the pet does now, or None. Pure: reads `demo_rng` without stepping it. Skips every
/// candidate whose starter would refuse it (`refusal_for`, `sleep_refusal`).
pub fn decide(s: &GameState) -> Option<Choice>;
/// The demo part of the wake view: bit 0 `decide(s).is_some()`, bit 1 help line (Task 5 sets it).
pub fn wake_view(s: &GameState) -> u8;
/// Remaining minutes of each running demo timer, for solver::timer_bound (plain timers).
pub fn timers(s: &GameState) -> [u32; 5];  // clock, whim, lazy, next, pause; u32::MAX when not running
// lifecycle.rs
pub fn local_minute() -> Option<u16>;  // ((unix_now + tz_offset_secs) / 60) % 1440
```
- `advance`, demo on: `clock_min` (when armed), `whim_min`, `lazy_min`, `next_min`, `pause_min` count down by `dt` (saturating); `day_min = (day_min + dt) % 1440`.
- `Visible` gains a 7th element, `demo::wake_view(self)` (0 when demo is off). `next_wake_tick`: at the top of each look-ahead iteration, `if c.demo.on { demo::settle(&mut c) }`, so the clone arms exactly what `update` will arm.
- `sync_clock` is called by `lifecycle` before every engine update it starts (`cycle`, `event_at`, `with_debug`) and by `on_wall_clock_set`. `day_min = (local_min - (now_tick - last_update_tick) / 6) mod 1440`.

**`settle` rules**, in this order (`r(n)` = `demo_rng % (n + 1)`, and `settle` steps `demo_rng` with `platform::xorshift32` once per draw):
1. `lazy_min == 0`: `lazy = r(DEMO_LAZY_MAX_PCT)`, `lazy_min = DEMO_LAZY_MIN`.
2. Clock: if `!clock_armed`:
   - `day_min == None` and asleep and energy ≥ `DEMO_RISE_ENERGY_PCT`: arm `clock_min = DEMO_RISE_BASE_MIN + r(DEMO_RISE_SPREAD_MIN)` (own rhythm).
   - `day_min == Some(m)`, awake: in the night window (`DEMO_BED_AT_MIN` to `DEMO_UP_AT_MIN`, wrapping midnight) and energy < `DEMO_RISE_ENERGY_PCT`: `clock_min = 0` (bed at once, after any pause); otherwise minutes until the next `DEMO_BED_AT_MIN + r(DEMO_BED_SPREAD_MIN)` (in the night window at full energy that is the next evening, so a pet woken at night does not flap in and out of bed).
   - `day_min == Some(m)`, asleep: in the night window or within `DEMO_BED_SPREAD_MIN` before it: minutes until `DEMO_UP_AT_MIN + r(DEMO_UP_SPREAD_MIN)`; otherwise (a daytime sleep that no whim nap armed) `DEMO_EDGE_NAP_MIN`.
   - Set `clock_armed` when one of these armed.
3. Soft delay: if some soft row of `decide` is true and `!soft_armed`: `next_min = DEMO_REACT_BASE_MIN + r(DEMO_REACT_SPREAD_MIN)`, `soft_armed = true`. If `soft_armed` and no soft row is true: `soft_armed = false`, `next_min = 0`. While booze > `DEMO_EDGE_BOOZE_PCT` the delay is the dry-out interval (`DEMO_DRY_*`) instead.
4. Whim: `whim_min == 0` and the whim row picks nothing: `whim_min = DEMO_WHIM_BASE_MIN + r(DEMO_WHIM_SPREAD_MIN)`.
- Any sleep ↔ wake transition (`fall_asleep`, `advance::wake`, the engine's self-wake) clears `clock_armed`, so the next `settle` arms for the new state. A whim nap arms its own clock in Task 4.

**`decide`** (short names drop `DEMO_` and `_PCT`, e.g. `WAKE_FULL` is `DEMO_WAKE_FULL_PCT()`): `None` unless demo is on, `phase == Active`, not hibernating, no action running, `pause_min == 0`. Then the first row that applies and is not refused (lines are levels; `lazy` is in percent, as `lvl(lazy)`):

| # | When | Choice |
|---|---|---|
| 1 | asleep and (`clock_armed && clock_min == 0` or fullness < `WAKE_FULL`) | `Wake` |
| 2 | asleep | none |
| 3 | `is_diabetic_unmedicated()` | `Insulin` |
| 4 | `is_alcoholic_untreated()` | `Rehab` |
| 5 | fullness < `EDGE_FULL` | `Eat(Apple)` if weight > `LIGHT_FOOD`, else `Eat(Pizza)` |
| 6 | hydration < `EDGE_WATER` | `Drink(Water)` |
| 7 | energy < `EDGE_ENERGY` | `Sleep` |
| 8 | health < `MED_HEALTH` | `Medicine` |
| 9 | booze > `EDGE_BOOZE` | `Drink(Water)` when `soft_armed && next_min == 0`, else none (rows 10 to 13 wait) |
| 10 | `clock_armed && clock_min == 0` | `Sleep` |
| 11 | fun − lazy > `WIRED_FUN` | `Relax` |
| 12 | `soft_armed && next_min == 0`: the first soft row that is true: hydration + lazy < `SOFT_WATER` → drink draw; fullness + lazy < `SOFT_FULL` → food draw; `day_min == None` and energy + lazy/2 < `SOFT_ENERGY` → `Sleep`; dirt − lazy > `SOFT_DIRT` → `Wash`; fun + lazy < `SOFT_FUN` and energy > `PLAY_ENERGY` and not injured → `Play`, or `Game(MiniGame::ALL[rng % 5], rng % 3)` with chance `PICK_GAME` | as listed |
| 13 | `whim_min == 0`: booze < `WHIM_BOOZE_LO` → beer with chance `WHIM_BEER_LO` (never while the dependence counter > `BEER_STOP`), else `Eat(Cake)` (skipped when weight > `LIGHT_FOOD`); booze < `WHIM_BOOZE_HI` and the counter ≤ `BEER_STOP` → beer with chance `WHIM_BEER_HI`; energy < `NAP_ENERGY` and fun < `NAP_FUN` → `Nap` | as listed |

Food draw (`rng % 100`): apple below `PICK_APPLE`, salad below +`SALAD`, pizza below +`PIZZA`, frikandel below +`FRIK`, else cake. Weight > `LIGHT_FOOD`: pizza, frikandel and cake become salad. Weight < `RICH_FOOD`: apple and salad become pizza. Drink draw: water below `PICK_WATER`, cola below +`COLA`, else beer; beer becomes cola while booze > `WHIM_BOOZE_LO`. The prototype's extra "15 % cake on top" of a meal is dropped (planner's call: the pet cannot chain two meals in one minute, and it pushed weight up; see "Needs the owner" (d)). Each soft row and each whim row skips refused choices and falls through to the next row. The dependence counter is the existing field that the onset at `advance` 143 reads; use it, add no new field.

`demo::levels` adds, per need, the lines rows 5 to 13 read (with the current `lazy`): Full `WAKE_FULL`, `EDGE_FULL`, `SOFT_FULL − lazy`; Water `EDGE_WATER`, `SOFT_WATER − lazy`; Energy `EDGE_ENERGY`, `SOFT_ENERGY − lazy/2`, `PLAY_ENERGY`, `NAP_ENERGY`, `RISE_ENERGY`; Fun `WIRED_FUN + lazy`, `SOFT_FUN − lazy`, `NAP_FUN`; Dirt `SOFT_DIRT + lazy`; Booze `EDGE_BOOZE`, `WHIM_BOOZE_LO`, `WHIM_BOOZE_HI`; Weight `LIGHT_FOOD`, `RICH_FOOD`; plus the Task 2 floors. Health lines are in `sig` (Task 2).

- [ ] **Step 1: write the failing tests:**
```rust
// mod policy
#[test] fn decide_follows_the_table() {
    // one state per row 1..13, each with only that row true, pause 0: decide == the row's Choice
    // (rng fixed so the draws are known: e.g. demo_rng chosen so rng % 100 == 10 -> Eat(Apple))
}
#[test] fn demo_never_takes_a_refused_choice() {
    // 2000 seeded random demo states: if let Some(c) = decide(s), the starter's refusal for c is None
    // (refusal_for for actions, sleep_refusal for Sleep/Nap, is_sleeping for Wake)
}
#[test] fn decide_waits_for_the_player_and_for_actions() { /* pause_min 5 -> None; active_action Some -> None; demo off -> None */ }
#[test] fn a_heavy_pet_eats_light() { /* weight 60 %: every food draw over rng 0..100 is Apple or Salad; whim never Cake */ }
#[test] fn bedtime_follows_the_clock() {
    // day_min 21:00, awake, energy 60: settle arms clock_min in 60..=150
    // day_min 01:00, awake, energy 60: clock_min == 0 -> decide == Sleep
    // day_min 01:00, awake, energy 98: clock_min until next 22:00..23:30
    // day_min 23:00, asleep: clock_min until 07:00..08:30
}
#[test] fn without_a_clock_the_pet_keeps_its_own_rhythm() {
    // day_min None, awake: clock never armed; energy + lazy/2 < SOFT_ENERGY arms the soft delay, then Sleep
    // asleep, energy reaches 97 %: clock armed 10..=120, then Wake
}
#[test] fn setting_the_clock_late_arms_the_bedtime() { /* None for 3 h, then sync_clock(…, Some(20:00)): next settle arms */ }
#[test] fn settle_never_changes_what_the_player_sees() { /* 2000 random demo states: visible() before == after settle */ }
// mod wake
#[test] fn demo_wake_is_exact_on_random_states() {
    // like engine_v3 status_wake::wake_is_exact_on_random_states, 2000 seeds, demo on, random lazy,
    // timers, day_min Some or None: settle a clone; next_wake_tick equals the first minute
    // (stepping a settled clone one minute at a time, settling after each) where visible() changes
}
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo policy::` and `... wake::`.
- [ ] **Step 3: implement** `Choice`, `settle`, `decide`, `wake_view` (bit 0 only), `timers`, the full `levels`, `sync_clock`, `local_minute`, the clock disarm in the sleep ↔ wake paths, the `next_wake_tick` settle.
- [ ] **Step 4: run them and see them pass.** `... --test engine_v3_demo policy::`, `... wake::`, and `... --test engine_v3 status_wake::`: PASS.
- [ ] **Step 5: commit** `feat(bornpets): demo policy choice as solver events`.
- [ ] **Step 6: V.**

### Task 4: the pet acts; the player steps in

Spec 8.4: player actions are free, a 30-minute policy pause follows a player action, `thanks!` when the player helped.

**Files:**
- Modify: `src/game/engine/demo.rs` (`run`, `act`, `Note`), `src/game/engine/mod.rs` (`update`, `finish_action`, `start_action`, `sleep`, `wake`, `award_inspiration`, transient fields), `src/game/engine/causes.rs` (`Cause::Thanks`).
- Test: `tests/engine_v3_demo.rs` `mod acts`, `mod split`.

**Interfaces:**
- Consumes: Task 3 `settle`, `decide`, `Choice`.
- Produces:
```rust
// demo.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Note { Ate(FoodKind), Drank(DrinkKind), OneTooMany, Napping, OffToBed, UpAndAbout, Washing, Played, Rested, Medicine, Insulin, Rehab }
impl Note { pub fn text(self) -> &'static str; }  // see "On-screen words", ≤ 21 columns
/// settle, decide, act, at most 4 times, until decide is None. Returns the number of acts.
pub fn run(s: &mut GameState) -> u8;
// mod.rs, GameState (transient)
pub(crate) self_care: bool,                              // true only inside demo::act
pub(crate) note: Option<(demo::Note, status::StatusLine)>, // the note and the line it was taken on
/// Apply the running action's dose now (split out of consume_action_ticks; that one calls it).
fn finish_action(&mut self);
// causes.rs
Cause::Thanks   // what "thanks!", why "that helped"; not a warning; pack code 18 (next free)
```
- `act(s, c)`: sets `self_care`, calls the starter for `c` (`feed`, `drink`, `sleep`, `wake`, `wash`, `play`, `relax`, `heal`, `medicate`, `rehab`, or `award_inspiration(game, grade)` for `Game`), then `finish_action()` for timed starters, so the dose lands in the same minute. `Nap` = `sleep()` then `clock_min = DEMO_NAP_BASE_MIN + r(DEMO_NAP_SPREAD_MIN)`, `clock_armed = true`. A whim act re-arms `whim_min`; a soft act clears `soft_armed`. Steps `demo_rng` once. Sets `note` to the matching `Note` and the status line after the act (`OneTooMany` when the act queued `Cause::OneTooMany`). Clears `self_care`. In the simulator, counts one self-care act (`sim::note_self_care()`).
- `update`: after `consume_action_ticks` and after each `advance` in the segment loop, `if self.demo.on { demo::run(self); }`. Nothing else in the loop changes: the pet's acts consume no ticks.
- The player: when demo is on and `!self_care`, `start_action`, `sleep`, `wake` and `award_inspiration` set `pause_min = DEMO_PAUSE_MIN` and clear `note`. When a player's action ends (`finish_action` with `!self_care`) and its dose lowered `regime::mood_penalty` and queued no warning cause, push `Cause::Thanks`.

- [ ] **Step 1: write the failing tests:**
```rust
// mod acts
#[test] fn the_pet_eats_when_hungry() {
    // demo, day_min Some(12:00), fullness 19 %: update one minute -> fullness rose, money unchanged,
    // active_action None, note == Some((Ate(Pizza), _)), age advanced exactly 1 minute
}
#[test] fn the_pet_uses_the_real_doses() {
    // two clones at the same state: A gets demo::act(Eat(Apple)); B gets feed(Apple) + finish_action:
    // every need equal (DRY: same starter, same dose)
}
#[test] fn the_player_pauses_the_pet() {
    // demo, thirsty pet: player drink(Water) -> pause_min == 30; update 29 min: no self-care act;
    // minute 30 onwards: the policy runs again
}
#[test] fn helping_gets_a_thanks() { /* hungry demo pet, player feed(Apple), action ends: take_causes contains Thanks */ }
#[test] fn no_thanks_for_a_bad_dose() { /* full demo pet, player feed(Pizza) -> FoodComa, no Thanks */ }
#[test] fn run_settles_to_nothing_to_do() { /* 2000 random demo states: after run, decide == None, run <= 4 */ }
#[test] fn a_whim_nap_ends_on_its_own() { /* whim_min 0, energy 50, fun 40: Nap; clock_min in 40..=100; Wake when it runs out */ }
#[test] fn the_badge_wakes_when_the_pet_acts() {
    // demo pet 10 min before the soft delay ends: next_wake_tick lands on that minute; update there
    // -> one act; next_wake_tick after it is later
}
// mod split
#[test] fn demo_update_is_split_invariant() {
    // 200 seeds: one update over 3 days vs the same span cut at 50 random ticks: equal snapshots
    // (needs, mood, health, demo block, demo_rng, note, tick_rem, age)
}
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo acts::` and `... split::`.
- [ ] **Step 3: implement** `run`, `act`, `Note`, `finish_action`, the player pause, `Thanks` (with its `what`/`why`, `PLAIN`, `pack`).
- [ ] **Step 4: run them and see them pass.** `... --test engine_v3_demo acts::`, `... split::`, `... --test engine_v3 causes::` and `... --test engine_v3 remainder` (the tick remainder tests at 1666 to 1715 stay green): PASS.
- [ ] **Step 5: commit** `feat(bornpets): the demo pet looks after itself`.
- [ ] **Step 6: V.**

### Task 5: a calm screen

Spec 8.3: no red, no toasts, no buzzer, no LED; notes on the status line; `could use a hand` in red below 35 % health.

**Files:**
- Modify: `src/game/engine/status.rs` (`danger` false in demo), `src/game/engine/demo.rs` (`wake_view` bit 1), `src/game/lifecycle.rs` (`pet_line`, `log_causes`, `check_severity_transition`, onset alerts, `money_enabled`), `src/game/mod.rs` (`PetLine` drawing, `DEMO` tag, `status_wants_full_refresh`).
- Test: `src/game/mod.rs` tests, `src/game/lifecycle.rs` tests, `tests/engine_v3_demo.rs` `mod calm`.

**Interfaces:**
- Consumes: Task 4 `note`, `Note::text`; plan 2 `draw_status_line`, `test_canvas::Canvas`.
- Produces:
```rust
// engine mod.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PetLine { Status(status::StatusLine), Note(demo::Note), Help }
/// What sits under the pet: Help when demo and health < DEMO_HELP_HEALTH_PCT; else the note while the
/// status line still equals the one it was taken on; else the status line.
pub fn pet_line(&self) -> PetLine;
impl PetLine { pub fn red(&self) -> bool; }  // Help, or Status with danger
// lifecycle.rs
pub fn pet_line() -> Option<engine::PetLine>;  // cheap read, no update
pub fn is_demo() -> bool;                      // false without a game
// game/mod.rs
pub fn draw_pet_line<D: DrawTarget<Color = TriColor>>(display: &mut D, line: &PetLine) -> Result<(), D::Error>;
```
- `status_line` in demo: `danger: false` (the words stay). `wake_view` bit 1 = help line, so its flip is a wake (its health line is in `sig` since Task 2). The note needs no wake: it is set at an act (already a wake) and ends when the status line changes (already a wake).
- `draw_pet_line` replaces the `draw_status_line` call: a note and the help line use the same position and backing; Help is bold red, a note bold black. `status_wants_full_refresh` reads `pet_line().is_some_and(|l| l.red())`.
- `DEMO` is drawn bold, right-aligned at `HAX_POS`, where HAX is drawn when `money_live`. `lifecycle::money_enabled()` returns `money_live()`, so menu prices and the battle view hide HAX in demo with no further change.
- Calm rules in `lifecycle`: `check_severity_transition` and the diabetes and dependence onset alerts play no buzzer while demo is on (the takeover screens still show). `log_causes` logs every cause in the diary; it toasts only while `!is_demo() || pause_min > 0` (the player just stepped in, so the toast answers them). The pet's own self-care is not a diary entry; only causes are.

- [ ] **Step 1: write the failing tests:**
```rust
// tests/engine_v3_demo.rs mod calm
#[test] fn the_demo_line_is_never_red() { /* demo, fullness 2 %: status_line().danger == false; demo off: true */ }
#[test] fn help_line_below_35() { /* demo, health 34 %: pet_line() == Help, red(); health 36 %: not Help */ }
#[test] fn a_note_ends_when_the_line_changes() {
    // act Eat(Apple): pet_line == Note(Ate(Apple)); advance until status_line changes: pet_line == Status(_)
}
#[test] fn help_flip_is_a_wake() { /* demo, health 36 % falling: next_wake_tick == first minute health < 35 % */ }
#[test] fn notes_fit() { /* every Note: text().len() <= 21; Note::OneTooMany ends with "..." */ }
// src/game/lifecycle.rs tests (lock_test_state)
#[test] fn pet_causes_do_not_toast() { /* demo pet's act queues FoodComa: cycle() -> diary has it, no cause toast */ }
#[test] fn player_causes_toast_in_demo() { /* player feed(Pizza) on a full demo pet: cycle() after the end -> toast FoodComa */ }
#[test] fn demo_never_buzzes() { /* severity rising in demo: the buzzer hook is not called (use the test seam the existing buzz tests use) */ }
// src/game/mod.rs tests
#[test] fn demo_tag_replaces_hax() { /* Canvas, demo pet with money 50: "DEMO" pixels right of x 120 on the HAX row, no digits */ }
#[test] fn help_line_is_red() { /* draw_pet_line(Help): RED pixels in y 96..=110 */ }
#[test] fn full_refresh_follows_the_pet_line() { /* demo, fullness 2 %: false; demo, health 30 %: true */ }
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo calm::` and `... --lib game::`.
- [ ] **Step 3: implement** as above. Remove `lifecycle::status_line` if `pet_line` leaves it without a reader (R15).
- [ ] **Step 4: run them and see them pass.** Same two commands (the `--lib` run skips `name_screen::tests::drops_control_and_beyond_latin1`). Run `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo sim -j 2` once with a demo pet: the note is legible over the sprite, `DEMO` sits where HAX was.
- [ ] **Step 5: commit** `feat(game): calm demo screen with notes and a help line`.
- [ ] **Step 6: V.**

### Task 6: the switch, the hint and the docs

Spec 8.5: "Demo mode: on/off" on the game settings screen. The hint gives the player something to do for a pet that cares for itself.

**Files:**
- Modify: `src/menu.rs` (`BORNAGOTCHI_ITEMS` 7 → 8), `src/game/lifecycle.rs` (`toggle_demo`, `demo_menu_label`), `src/game/engine/hint.rs` (`Subject::Stiff`), `GAME.md`, `USER_GAMES.md`.
- Test: `src/menu.rs` tests, `src/game/lifecycle.rs` tests, `tests/engine_v3_demo.rs` `mod hint`.

**Interfaces:**
- Consumes: Task 1 `set_demo`; plan 2 `hint`, `remedy_reason`.
- Produces:
```rust
// lifecycle.rs
pub fn toggle_demo() -> Result<bool, engine::doses::Reason>;  // new state, or why not; saves at once
pub fn demo_menu_label() -> &'static str;
//   "Demo mode: on" | "Demo mode: off" | "Demo: fix me first" | "Demo: no pet" (egg, gone or none)
// hint.rs
Subject::Stiff   // "I feel stiff."; remedies [Exercise]
```
- The menu row sits after "Mode: Classic/Casual" and before Reset Pet; pressing it calls `toggle_demo`. A refused toggle changes nothing; the label already says why (R24 text for FixMeFirst).
- `hint`: when demo is on, the pet is awake, the subject would be an info word (no problem word, not WornOut) and fitness < `CEILING_FIT_STEP3_PCT`, the subject is `Stiff`. Ask follows the existing rule, so a tired or injured pet says why instead of asking (`Ask::Refused`).
- Docs: `GAME.md` gets a "Demo mode" section: what the pet does by itself, that it never leaves, never exercises, follows the clock (or its own rhythm without one), the `DEMO` tag, frozen HAX, notes, the help line, the 30-minute pause and `thanks!`, and the settings row. `USER_GAMES.md` BornPets: one paragraph and the settings row.

- [ ] **Step 1: write the failing tests:**
```rust
// tests/engine_v3_demo.rs mod hint
#[test] fn a_stiff_demo_pet_asks_for_exercise() { /* demo, content, fitness 50 %: Subject::Stiff, Ask::Do(Exercise) */ }
#[test] fn demo_hint_is_never_refused() { /* 2000 random demo states: Ask::Do(r) => remedy_reason(r) is None */ }
#[test] fn stiff_fits() { /* subject_line(Stiff).len() <= 18 */ }
// src/game/lifecycle.rs tests
#[test] fn the_toggle_says_why() { /* health 30 %: toggle_demo() == Err(Refused(FixMeFirst)), label "Demo: fix me first";
                                      health 60 %: Ok(true), label "Demo mode: on", is_demo() */ }
// src/menu.rs tests
#[test] fn game_settings_has_the_demo_row() { /* BORNAGOTCHI_ITEMS.len() == 8; every demo label <= 20 columns */ }
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo hint::` and `... --lib menu:: game::lifecycle::`.
- [ ] **Step 3: implement**, then the docs.
- [ ] **Step 4: run them and see them pass.** Same commands, plus `... --test engine_v3 hint::`: PASS.
- [ ] **Step 5: commit** `feat(game): demo mode switch and a stiff hint`.
- [ ] **Step 6: V.**

### Task 7: simulator runs for the demo pet

Spec 18.3 gives the target numbers; 18.9: "Tune it in plan 3". This task builds the measuring tools and writes the long runs. It runs only a 2-day smoke locally; the long runs execute in Task 8.

**Files:**
- Modify: `src/game/engine/sim.rs`, `.github/workflows/tests.yml`, `tests/engine_v3_casual.rs`.
- Test: `tests/engine_v3_demo.rs` `mod balance`.

**Interfaces:**
- Consumes: Tasks 1 to 6.
- Produces:
```rust
// sim.rs
#[derive(Clone, Copy, Debug, Default)]
pub struct Setup { pub demo: bool, pub clock: bool }   // clock: day_min starts at 08:00 and follows the run
pub fn run_with(routine: Routine, days: u32, driver: Driver, seed: u64, setup: Setup) -> Run; // run_full == run_with(.., Setup::default())
pub fn note_self_care();                 // thread-local counter, called by demo::act
// Outcome gains:
pub thriving_pct: u8, pub fine_pct: u8, pub off_pct: u8, pub poorly_pct: u8, pub miserable_pct: u8, // time shares of the mood bands
pub self_care_per_day: f32, pub causes_per_day: f32,
pub max_weight_pct: u8, pub min_fitness_pct: u8,
pub night_asleep_pct: u8,   // share of 00:00..06:00 asleep (clock runs only)
pub day_asleep_pct: u8,     // share of 10:00..20:00 asleep
pub ever_leaving: bool, pub diabetic: bool,
// Routine gains:
Routine::Nobody,   // no player at all
Routine::Helper,   // at 18:00 each day: one apple, one water, one exercise, one wash
```
- `note_segment` takes a before/after snapshot (mood, health, weight, fitness, `is_sleeping`, `day_min`, phase) instead of the four numbers, so the band shares and the asleep shares are measured per segment. The oracle driver steps one minute at a time through `update`, so it runs the policy the same way; that is the test oracle the owner allows.
- CI: add `engine_v3_demo` to the matrix, run as `cargo test --features simulator --test engine_v3_demo -- --include-ignored`, and add it to the `others` skip list.

**Long runs** (all `#[ignore]`, run by CI in Task 8; Classic unless noted; 5 seeds each):
```rust
#[test] #[ignore] fn demo_alone_for_30_days() {
    // Nobody, clock, Delta: !ever_leaving; avg mood 50..=80; miserable <= 2 %; thriving >= 5 %;
    // min health >= 40 % (floor 25 is a safety net, not the routine); min fitness >= 45 %;
    // max weight <= 70 %; end weight 38..=66 %; !diabetic; self-care 10..=20 per day;
    // engine wakes <= 60 per day; causes <= 6 per day; night asleep >= 85 %; day asleep <= 20 %
}
#[test] #[ignore] fn demo_without_a_clock_for_30_days() { /* Nobody, no clock: same health, weight and wake bounds; no sleep-share bounds */ }
#[test] #[ignore] fn demo_with_a_helper_for_30_days() { /* Helper, clock: avg mood >= the Nobody run's for the same seed; max weight <= 70 % */ }
#[test] #[ignore] fn demo_delta_matches_oracle() { /* Nobody and Helper, clock, 10 days, 3 seeds: Delta and Oracle Outcomes equal (strong agreement, as oracle::delta_matches_oracle_all_routines) */ }
// focused, not ignored:
#[test] fn demo_smoke_two_days() { /* Nobody, clock, 2 days, seed 1: alive, !ever_leaving, self-care > 0 */ }
// tests/engine_v3_casual.rs:
#[test] fn casual_demo_alone_for_10_days() { /* Nobody, clock: !ever_leaving, min fitness >= 40 %, max weight <= 70 % */ }
```
- [ ] **Step 1: write the tests** (the smoke test fails: `run_with` missing).
- [ ] **Step 2: run and see the smoke fail.** `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo demo_smoke`.
- [ ] **Step 3: implement** `Setup`, `run_with`, the counters, the snapshots, the routines, the CI entry.
- [ ] **Step 4: run the smoke and see it pass.** Same command. Do not run the ignored tests locally.
- [ ] **Step 5: commit** `test(bornpets): demo balance runs in the simulator`.
- [ ] **Step 6: V.**

### Task 8: regression and integration on CI

One push, the whole suite. Nothing is changed in this task unless a step says so.

**Files:** none (report only), unless step 5 applies.

**Interfaces:**
- Consumes: Tasks 1 to 7.
- Produces: the regression report (in the task report, not a file): every CI job's result, the failing test names with their assert messages, the demo numbers from the long runs (print them with `eprintln!` in the long tests, which CI shows with `--include-ignored`; add `-- --nocapture` to the demo matrix line if they are not shown), and the SSD1680 release image size.

- [ ] **Step 1:** run locally, one command at a time: `CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo test -j 2 --features simulator --test engine_v3_demo` (the ignored long runs stay off), then `... --test engine_v3 split_invariance::`, `... --test engine_v3 status_wake::`, `... --test engine_v3 oracle::`. Fix nothing yet; note failures.
- [ ] **Step 2:** `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh git push github ranzbak/bornpets-v3-demo`.
- [ ] **Step 3:** `gh run list -R ranzbak/bornhack-firmware-2026-ce -b ranzbak/bornpets-v3-demo -L 1 --json databaseId,headSha`. The `headSha` must equal `git rev-parse HEAD`.
- [ ] **Step 4:** `gh run watch <id> -R ranzbak/bornhack-firmware-2026-ce --exit-status`, then `gh run view <id> -R ranzbak/bornhack-firmware-2026-ce --json conclusion,jobs`. For each failed job: `gh run view <id> -R ranzbak/bornhack-firmware-2026-ce --log-failed`.
- [ ] **Step 5:** check the demo numbers against spec 18.3 (mood 57 to 65, thriving 10 to 19 %, poorly 3 to 12 %, self-care 15 to 17 a day, wakes 32 to 35 a day plus one per act) and against the long-run asserts. The SSD1680 image must be ≤ 960 KiB. If it is over, removing Lights Out goes to Task 9 as its own item.
- [ ] **Step 6:** report: green or the list of failures, the numbers, the image size. If everything is green, Task 9 only records "nothing to fix" and the final numbers.

### Task 9: bug-fix round

Fix what Task 8 found, one item at a time.

**Files:** as each fix needs.

**Interfaces:**
- Consumes: the Task 8 report.
- Produces: fixes; the tuned `DEMO_*` values; a final green CI run.

For each failure:
- [ ] **Step 1:** reproduce it with a focused test (an existing one, or a new short one in the matching `tests/engine_v3_demo.rs` module, bounded to at most 10 simulated days so it runs locally). See it fail.
- [ ] **Step 2:** fix the cause (superpowers:systematic-debugging: root cause first, no tuning around an engine bug).
- [ ] **Step 3:** run that focused test and the module it sits in. See them pass.
- [ ] **Step 4:** commit `fix(bornpets): <what>` (one commit per item).

Tuning rules (balance failures only, after the engine is shown correct):
- Weight over the bound: lower `DEMO_LIGHT_FOOD_PCT` in steps of 2, down to 50. Then lower `DEMO_PICK_PIZZA_PCT` by 5 and raise `DEMO_PICK_APPLE_PCT` by 5. Report the final values.
- Mood under the band: raise `DEMO_SOFT_FUN_PCT` by 3 first, then `DEMO_SOFT_FULL_PCT`.
- Too many wakes: widen `DEMO_REACT_SPREAD_MIN` before touching any line.
- Night share under 85 %: check the clock arming first (`bedtime_follows_the_clock`), then the Classic `DEMO_BED_AT_MIN`.
- Every tuned value changes both presets where the Casual one depends on it, regenerates the template (`BPETS_REGEN=1`, Task 1) and updates the `USER_GAMES.md` key list in the same commit.
- Flash over 960 KiB: remove the Lights Out mini-game in its own commit (owner's allowance), with its menu row, cooldown, docs and tests.

After the last fix:
- [ ] **Step 5:** push and watch CI as in Task 8 steps 2 to 4. Repeat this task until CI is green. Report the final demo numbers, the tuned values and the image size.

## On-screen words

| Where | Text |
|---|---|
| Settings row | `Demo mode: on` / `Demo mode: off` / `Demo: fix me first` / `Demo: no pet` |
| Reason (`Refusal::FixMeFirst`) | `fix me first` |
| Tag on the HAX row | `DEMO` |
| Help line (red) | `could use a hand` |
| Notes (`Note::text`) | `had an apple`, `had a salad`, `had some pizza`, `had a Frikandel spec`, `had some cake`, `had some water`, `had a cola`, `fancied a beer`, `had some wine`, `had a whiskey`, `one too many...`, `napping`, `off to bed`, `up and about`, `having a wash`, `had a play`, `had a rest`, `took medicine`, `took insulin`, `went to rehab` |
| Cause toast | `thanks!` / `that helped` |
| Hint | `I feel stiff.` (ask: exercise) |

The demo pet never drinks wine or whiskey by itself; their notes exist because `Note::Drank` takes any `DrinkKind`. Spec 8.3 words kept: `had an apple`, `had some water`, `napping`, `fancied a beer`, `having a wash`, `one too many...`, `could use a hand`. `Ate(Cake)` after a whim reads `had some cake`; the whim beer reads `fancied a beer`, a soft-row beer too (planner's call: one word per drink).

## Self-review

**Spec coverage:**

| Spec item | Where |
|---|---|
| 8.1 policy table, temperament, soft delay, whims, never exercises | T3 (`decide`, `settle`), T4 (acts) |
| 8.1 clock bedtime and getting up; own rhythm without a clock | T3 (`settle` rule 2, `sync_clock`), T7 (night and day shares) |
| 8.2 edge rules, health floor 25, no leaving, fitness floor, no dependence, no care score | T2, T3 rows 3 to 9 |
| 8.2 "self-raised" after a quarter of the life | T1 (`self_raised`, counter); recording it is plan 8 |
| 8.3 calm: no red, no toasts, no buzzer, no LED; notes; help line | T5 (no code drives the LED from the game today, so "no LED" needs no change) |
| 8.4 player help free, 30-min pause, `thanks!` | T1 (free: `money_live`), T4 (pause, Thanks), T5 (toast rule) |
| 8.5 settings row, `DEMO` tag, save byte, HAX frozen, guard | T1, T5, T6 |
| 8.5 beacon flag bit | not done: plan 5 owns the beacon; T5 gives it `lifecycle::is_demo` |
| 13.3 policy as solver events, act as event | T3 (`levels`, `timers`, `wake_view`), T4 (`run` in the segment loop) |
| 15.4 dependence never set in demo | T2 |
| 17 rows 11 to 15 | T1 to T6 |
| 18.3 demo results; 18.8 floor; 18.9 tuning | T7 (runs), T8 (measure), T9 (tune) |
| Owner: weight tuned (a run ended at 98 %) | T3 (heavy pet eats light, cake-on-top dropped), T7 bound ≤ 70 %, T9 procedure |
| Owner: test policy (focused per task, one regression task, one fix round) | V step, T8, T9 |

**Resolved conflicts and gaps (planner's calls):**
1. **The pet's own doses are instant.** A timed action started mid-update would have to spend whole minutes, but the existing rule "the idle remainder is not spent on an action" (tests at `tests/engine_v3.rs` 1666 to 1715) makes split invariance impossible for an action that starts between two updates. So `act` calls the real starter and finishes it in the same minute: same refusals, same dose, no ticks. The screen shows a past-tense note ("had an apple") instead of the eating animation, which also saves e-paper refreshes. See "Needs the owner" (c).
2. **Save.** Demo state uses the reserved tail (bytes 149 to 159), not a new format: zeros already mean "off", so a plan-2 save upgrades by doing nothing, and version 3 / 177 bytes stay. 11 bytes leave the 17 plan 6 needs. Demo time is saved in whole hours (u16, 7.4 years).
3. **The RNG is not saved.** It only draws future delays; after a reboot it restarts from `age_ticks` and `generation`, so a reboot changes which apple comes next, not the pet's state.
4. **Bedtime without a clock.** `day_min` is RAM only. Until the wall clock is set the pet uses its own rhythm (spec 8.1's sleepy soft row and the 97 % getting-up rule); `sync_clock` switches it to the clock at the next boundary.
5. **Night window and a woken pet.** A pet the player wakes at night goes back to bed after the pause unless it is full of energy, in which case it stays up until the next evening. This avoids flapping between beds.
6. **Floors pin, they do not lift.** A stat below its floor when demo is turned on stays where it is until care lifts it.
7. **Demo pet's causes.** Logged in the diary (they are true), toasted only while the player's pause runs. Self-care acts are notes, not diary entries, so the diary is not filled with apples.
8. **Thanks without a forced animation.** The `Happy` animation is state-derived; forcing it would add a wake. The `thanks!` toast carries it.
9. **Self-played mini-game.** `Game(game, grade)` is `award_inspiration` with no animation and no HAX; it sets that game's cooldown like a played one.
10. **"Cake on top" dropped** (prototype: 15 % extra cake after a meal). One act per minute, and it was the main weight driver.
11. **Casual soft lines** shift 5 points with the Casual bands; the floor is 40 there.
12. **Thresholds count.** 52 new keys. Each is a balance value (rows, delays, draws, the clock window), so each is a `BPETS_V3.CFG` key. No new display const: the note and help texts are words, not numbers.
13. **Help line 35 %** is a `Thresholds` field (`DEMO_HELP_HEALTH_PCT`), since it is a condition threshold.
14. **The demo pet does not need the 12-hour boot catch-up cap changed.** Catch-up runs `update`, so the policy runs through it segment by segment.

**Needs the owner:**
- (a) Fitness floor: Classic 45 (the Classic fitness band starts at 43, so 40 would show "unfit"), Casual 40. Or 40 in both?
- (b) Bedtime and getting-up windows are fixed config keys (22:00 to 23:30, 07:00 to 08:30). Should the pet instead learn the player's hours from when the badge is used?
- (c) The pet's own care is instant, shown as a note, with no eating or washing animation. OK, or should self-care animate (more refreshes)?
- (d) Dropping the prototype's "cake on top", and the self-played mini-game without animation.
- (e) Should a demo note chirp the buzzer? This plan keeps demo fully silent.
- (f) Weight targets: max 70 %, end 38 to 66 % over 30 days. Are these the right bounds?
- (g) "Self-raised" is only counted here (`demo_min`, `self_raised`); recording it on the pet's record is plan 8. OK?

**Type consistency:** checked `Demo`, `Choice`, `Note`, `PetLine`, `settle`, `decide`, `run`, `levels`, `timers`, `wake_view`, `floor`, `sync_clock`, `day_min`, `self_care`, `note`, `money_live`, `set_demo`, `toggle_demo`, `Setup`, `run_with` across the tasks that consume them. `Visible` gains exactly one element (T3); T5 only sets its bit 1.

**Review Focus:** items 1 to 5 are pinned in T3 / T4, T4 / T8, T2 / T8, T1, and T3 / T5 / T6.

**Proportion:** 7 build tasks, each one engine or UI layer; the engine work (T1 to T4) carries the risk and most tests. The long balance runs are written once (T7) and executed once (T8), as the test policy asks.

**Flash:** no font, no sprite. 52 `Thresholds` fields (two presets, about 150 bytes each) and their key strings (about 1.2 KB), the policy code and about 400 bytes of words: an estimated 5 to 7 KB. Measured in Task 8.
