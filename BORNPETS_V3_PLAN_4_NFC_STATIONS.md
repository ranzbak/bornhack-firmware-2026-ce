# BornPets v3, plan 4: NFC stations (the magic places) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a badge held to a station reader gets a fix from a signed `fix s=<station> <items>` plaintext that names v3 needs (`full>60`, `clear=hangover`, `cd=240`, ...). The badge clamps every value into a safe range, gives a mood bump of +5, and remembers a cooldown per station. The three old phrases (`more food`, `more drugs`, `sleep like a bear`) stay as aliases, so readers in the field keep working. Every tap shows the player a toast: what the fix was, or why it was refused (`Too soon!` / `again in 4m`). Organisers get a checker and a sender for the new strings (`scripts/station.py`) and a preset list. The guide's chapter 11 explains it.

**Architecture:** parsing and applying are pure engine code. `engine::fix` turns the normalised text into a `Fix`, expanding an alias first. `GameState::station_fix` applies the fix: conditions first, then levels in message order, each clamped by `fix::safe_range`, then the mood bump. `game::station` is the dispatcher: it holds the RAM cooldown table (8 station ids), checks the pet, calls the engine through `lifecycle::with_game_now` (catch up to now, apply, the display loop recomputes the next wake), raises the toast and brings the game screen forward. It returns an `Outcome`, and `nfct.rs` maps it to a status word. A fix is an event like a button press: no polling, no step loop, no save byte.

**Tech stack:** Rust `no_std` firmware (embassy, embedded-graphics, FONT_7X13 / FONT_7X13_BOLD), host tests through the `simulator` feature, Python 3 stdlib for `scripts/station.py` (plus `cryptography` and `pyscard` only for signing and sending), Typst guide (`make guide`).

**Spec:** `BORNPETS_V3_DESIGN.md` §19 (19.1 to 19.7), with 18.8 rows 6 and 7 (nap tent and oversleep, clinic). Roadmap row 4 and the cross-plan rules in `BORNPETS_V3_ROADMAP.md`. Format exemplar: `BORNPETS_V3_PLAN_7_BACKGROUNDS.md`. Guide design: `BORNPETS_V3_GUIDE_DESIGN.md`. Protocol: `NFC_README.md`.

**Code base:** worktree `BornHack-Cyberegg-v3nfc`, branch `ranzbak/bornpets-v3-nfc`, from local `main` `a718cc0`. Anchors below are at `a718cc0`, except the lines marked "(battles)", which are on `ranzbak/bornpets-v3-battles` `6bcf7f3`. After the merge in "Before Task 1" the line numbers move: find each anchor by its name.
- `src/game/station.rs` 1 to 127: the whole dispatcher today (three phrases, three cooldown atomics, `normalize` with a 32-byte limit).
- `src/fw/nfct.rs`: `WriteOutcome` 43 to 56, `handle_signed` 377 (game block 410 to 433, with the screen switch at 418), `handle_ndef_write` 455 (opt-in station block 477 to 484).
- `src/game/engine/mod.rs`: `pub mod` list 13 to 30, `STATION_FOOD_FLOOR_PCT` / `STATION_BOOZE_CAP_PCT` 117 to 120, `GameState` 245 (`no_groggy` 284, `hard_mode`, `demo`), `player_acted` 792 (private), `wake_view_for_test` 1475, `next_wake_tick` 1498, `station_food` / `station_drugs` / `station_rest` 1853 to 1883.
- `src/game/engine/regime.rs`: `health_ceiling` 278, `health_damage` 303 (the damage lines this plan clamps against).
- `src/game/engine/needs.rs`: `HEALTH_DANGER_PCT` 24, `booze_danger()` / `strain_inj()` in the `band!` block 62 to 75.
- `src/game/engine/advance.rs`: the finished-rehab rule 183 to 187, the diabetes reversal 156 to 159, the `no_groggy` use 295 to 300.
- `src/game/engine/thresholds.rs`: field table (last field `FLOOR_PIZZA_DIRT_PCT` 393), `CLASSIC` (ends ~729), `CASUAL` (ends ~1062), `BORNPETS_CFG_KEYS` (ends ~1950), `clamp_u16` 1961, `clamp_pct` 1978, `validate` 1984.
- `src/game/engine/sim.rs`: `Act` 40, `perform` 440, routines `balanced` 737, `party` 816, `once_a_day` 890, `absent` 900.
- `src/game/mod.rs`: `Toast` 65 (station variants 76 to 83), `from_u8` 122, `message` 158, `STATION_COOLDOWN_SECS` 215, `show_toast` 371, `draw_cause_toast` 385, `screen_words` 425, `show_reason` 509, `full_screen_view_up` 517, `show_station_cooldown` 625, `HAX_POS` 656, `TOAST_POS` 659, `draw_toast` 799 (cooldown branch ~825), `draw_screen_game` Hatching branch 1026, Gone branch 1041, `draw_toast` call ~1112, `lock_test_state` 1309, test `the_cause_toast_clears_the_hax_balance` 1524.
- `src/game/lifecycle.rs`: `now_tick` 42, `can_use_station` 236, `set_state_for_test` 619, `hard_mode` 1247, `with_game_now` 1273, the `can_use_station` assert in a test at 1966.
- `src/game/battle_view.rs` (battles): `open_pre_battle` 458 to 473 closes Friends, Diary and Traits and switches to the game screen ("like an NFC bonus").
- `src/guide/data.rs` `fixed()`: rows `STATION_COOLDOWN` 146, `STATION_FOOD_FLOOR_PCT` 161, `STATION_BOOZE_CAP_PCT` 166; `dump` 372. `src/guide/shots.rs`: `SHOTS` 388, `cause-toast` shot 487.
- `tests/engine_v3.rs`: `mod station_aliases` 3681 to 3797, `preview_sleep_is_the_night_slept_until_rested` ~4333 (calls `s.station_rest()` at 4352), `mod status_wake` 2147 (`first_change_min` 2173, `wake_is_exact_on_random_states` 2319).
- `guide/chapters/11-friends-battles-stations.typ` (battles): `== Stations: the magic places` 221 to 254. `guide/chapters/05-care-actions.typ` (battles): toast table 545 to 568, station rows 563, 566, 567, the `wait 1:30` paragraph 574, the never-raised sentence 577. `guide/lib/social.typ` `station-row` 104.
- `.github/workflows/tests.yml` (battles): matrix line 28, `others` exclusion list, `battle_v3` arm, `guide` job last steps.
- Docs: `NFC_README.md` §4 (still lists `more inspiration` and the v1 effects), §5.1, §10; `USER_NFC.md` command table 15 to 26; `README.md` 272 to 285; `QUICKSTART.md` 183 to 190; `scripts/README.md`.
- Field reader: the BadgeCtl Android app, a separate git repo at `../../android_nfc/BadgeCtl` (`app/src/main/java/com/example/badgectl/MainActivity.kt` 78 to 89 arms four phrases, `handleResponse` ~293). It has **uncommitted local changes** (`git status`: `MainActivity.kt`, `activity_main.xml`, `AndroidManifest.xml`, `README.md` modified, `dist/` untracked).

## Owner questions

Most important first. The tasks are written for the recommendation; the affected steps carry the question's tag.

1. **[Q1] Readers talking to badges on old firmware.** Badges that never get this firmware ignore a `fix ...` string (`6A 82`, silent). Recommendation: the badge answers `69 85` ("understood, refused": cooldown, pet gone, no pet) and keeps `6A 82` for "not understood". A reader that gets `6A 82` for a preset that has an old-phrase equivalent (food, pharmacy, nap tent) sends the old phrase once more with a fresh challenge. So an old badge still gets its food. Without the new status word the fallback would let a new badge on cooldown take the same fix twice under the alias's id. Tasks 4, 6, 8.
2. **[Q2] Who updates the field reader (BadgeCtl)?** The spec says the reader tool gets the new strings. BadgeCtl lives in its own repo and has uncommitted local work. Recommendation: this plan ships `scripts/station.py` in this repo, with `check`, `presets`, `apdu` and `send` for a USB PC/SC reader. It adds a gated Task 8 for BadgeCtl: the eight presets as buttons and the Q1 fallback. Task 8 runs only after the owner has committed the local BadgeCtl changes and says go. The owner builds and installs the APK (`./gradlew assembleDebug`). Tasks 6, 8, 10.
3. **[Q3] `more inspiration`.** Today's BadgeCtl still has an Inspire button that sends `more inspiration`. Today's firmware ignores it (the drained stat is gone), and spec 19.3 lists only three aliases. Recommendation: a fourth alias `more inspiration` = `fix s=inspire fun>60` (fun raised to 60 %, inside the 33 to 73 % band, like the food alias). Otherwise the button would show `Unknown magic`. Tasks 1, 6, 7, 10.
4. **[Q4] Cooldowns forget on reboot.** Spec 19.7 keeps the table in RAM. So a player can power-cycle the badge to use the rare clinic (`cd=1440`) twice in a day. Recommendation: keep RAM as the spec says. The clinic is rare and operator-run, and the save's reserved bytes are spoken for by plans 3, 6 and 8. Task 4.

## Before Task 1 (assumptions)

- Plans 5 (Battles), 6 (Hard mode and money) and 8 (Long-term layer) are merged into `main` before this plan starts (owner order: NFC last). First step of Task 1: `git merge main` on `ranzbak/bornpets-v3-nfc`, then re-find every anchor above by name.
- Plan 5 keeps the screen switch of `battle_view::open_pre_battle` as on `6bcf7f3` (close Friends, Diary and Traits, then `set_active_screen(SCREEN_GAME)`). Task 3 moves those lines into one shared `game::bring_game_forward()`. `nfct.rs` is not touched by plan 5, so its switch at 418 is still there and Task 4 removes it.
- Plan 6 keeps `GameState::hard_mode` as the hard-mode flag and charges money only through the starters and its bill code. A station fix calls neither, so it is free (spec 19.4). Task 2 pins it: a fix changes no money field.
- Plans 5, 6 and 8 add `Thresholds` fields. This plan appends its four fields as one block after whatever field is last at merge time, in each list. It regenerates the template from the merged table.
- Plan 8 does not change `player_acted` or the demo pause. If it does, Task 2 follows plan 8's entry point for "player help" instead of calling `player_acted` directly.
- If a plan that merges later changes any of this, the implementer stops and reports instead of guessing.

## Global Constraints

Every task's requirements include this section.

**Owner rules:**
- The delta engine drives progress. No polling and no fixed-step loop. A fix is applied at one engine event: `with_game_now` catches up to now, applies, and the display loop recomputes `next_wake_tick` on the `TOAST_SIGNAL` wake. A step loop may exist only as the simulator's test oracle (`Driver::Oracle`).
- A refused action always tells the player why. Cooldown: `Too soon!` / `again in <time>`. Gone pet: `pet is gone`. Unreadable or unknown signed text: `Unknown magic` / `badge can't read it`. The only silent case is a badge with no game at all: the tap brings the start screen forward, and that screen is the answer (planner call 12).
- Stay a Tamagotchi variant; every need has a sweet spot. A station never moves a need past its health-damage line (Task 2 `safe_range`).
- Labels may be renamed for clarity. The toast texts below are this plan's decision, within the toast geometry.
- Do not touch `GAME.md` (the roadmap's final task rewrites it).
- Old badges and readers in the field keep working: the old phrases are aliases (Task 1), and [Q1] gives readers a fallback for badges on old firmware.
- **TDD, quality first.** Cheaper models implement, so every name, number and text below is the decision. Where the plan leaves a choice, it says so.
- **Test policy:** each task runs only its own focused tests locally. Long simulations are `#[ignore]` and run on GitHub CI only (Task 5). The full suite, both firmware builds, the flash gate and the guide build run once, in Task 9, on the GitHub mirror.
- **Local cargo:** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo ... -j 2`, one cargo command at a time (also `cargo check` and `cargo sim`). Never run `cargo fmt` on the whole tree. Never use `git stash`. Firmware is only *checked* locally, and only in Tasks 4 and 9: `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo check -j 2 --bin embassy --no-default-features --features embassy-1680-full --target thumbv7em-none-eabihf`, the same with `--features embassy-game`, and with `--features embassy-1680-full,nfc-plaintext-station`.
- **CI and pushing:** CI runs on the GitHub mirror only, and every job keeps `if: github.server_url == 'https://github.com'`. Push `ranzbak/bornpets-v3-nfc` to the `github` remote only (public mirror `ranzbak/bornhack-firmware-2026-ce`), never to codeberg, and only in Tasks 9 and 11. Prefix SSH git commands with `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh`.
- **Signed channel:** the APDU, challenge and signature scheme do not change (spec 19.2). Only the signed string and the status word for "understood, refused" change [Q1]. `src/signed_channel.rs` is untouched.
- **Saves:** none. The cooldown table is RAM (spec 19.7, [Q4]). `state_v3` and its reserved bytes are not touched.
- **Tunables:** four new `Thresholds` fields, each with a Classic and a Casual value and a `BPETS_V3.CFG` key (Task 2). The template `assets/to-badge/BPETS_V3.CFG` and the `USER_GAMES.md` key list are regenerated in the same commit. Protocol limits stay `const`: 96 bytes, 8-byte ids, 8 cooldown slots, the alias table.
- **Flash:** the SSD1680 release image is about 871 KiB of the 960 KiB gate before plans 5, 6 and 8. This plan adds an estimated 3 to 4.5 KiB (per-task estimates in each task, the total in Self-review). If space runs short, the owner allows removing the Lights Out mini-game (Task 11 only, its own commit).
- **Guide:** this plan rewrites the Stations section of chapter 11 and the station rows of chapter 5. Every on-screen word is marked with `w()`. Numbers come from `v()`. Preset and alias strings come from guide data, never typed (`lint_digits`). Run `make guide` in Task 7.
- Commit footer on every commit: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Carried from plans 1 to 7:** integer math only in `src/game/engine/` (except `sim.rs`); no `alloc` in engine code (`heapless` is fine); levels are `u16` with `P = 655` per percent and `MAX = 65535`; 1 engine minute = 6 ticks of 10 s; R15 (code this plan makes dead is removed in the same commit, no commented-out code); R24 (every refusal shows its reason); R26 ("BornPets v3"); R7/R13 (the known failures `name_screen::tests::drops_control_and_beyond_latin1` and the `signed_channel.rs` doctest are not yours).

**Toast geometry** (from `the_cause_toast_clears_the_hax_balance`): a station toast has two lines, like the cause toast. Line 1 is bold red at `TOAST_POS` (2, 36) and has at most **13 columns**, so it clears the widest HAX balance `999999 H` at the top right. Line 2 is red, one row lower (`TOAST_LINE_DY` 13), and has at most **21 columns** (it ends at x ≤ 150). Both lines sit on white backings.

| Toast | Line 1 | Line 2 |
|---|---|---|
| `StationToast::Fixed` | `Magic fix!` | `at <station>`, e.g. `at food1` |
| `StationToast::Thanks` (demo pet, awake) | `Magic fix!` | `thanks!` |
| `StationToast::Covered` (hard mode) | `Covered by` | `magic, not insurance` |
| `StationToast::TooSoon(secs)` | `Too soon!` | `again in <time>` (`doses::fmt_wait`: `45s`, `4m`, `3h59`, `24h00`) |
| `StationToast::Unknown` | `Unknown magic` | `badge can't read it` |
| Gone pet (existing `Reason::Gone`, one line) | `pet is gone` | |

**Status words** [Q1]:

| `Outcome` | SW | When |
|---|---|---|
| `Applied` | `90 00` | the fix was applied |
| `Refused` | `69 85` | understood, but on cooldown, pet gone, or no game |
| `Unknown` | `6A 82` | not a station message, unreadable, malformed, or nothing known (signed path) |
| `NotStation` | `6A 82` | plain NDEF text that is not a station message (opt-in unsigned path: becomes the profile, no toast) |

**Verify (V)**, the last step of Tasks 1 to 7:
1. Run the task's focused tests (the commands are in the task). Nothing else.
2. `git status --short` shows only the files the task names.
3. No push. Report the test names run and their result.

## Review Focus

The failure modes most likely to reach a player unnoticed, most likely first. Each is pinned by a test in the task that owns the code.

1. **A clamp makes a pet worse.** A station must never push a need into health damage. Its own clamp must also never pull a need the wrong way: the fit cap lowering a fit pet, or a stuffed pet's fullness being touched by `full>60`. Pinned in Task 2 by `no_fix_raises_health_damage` (3000 random fixes on random states), `the_clamp_never_works_against_the_item`, `fit_is_capped_but_never_lowered` and `a_pet_outside_the_range_is_only_helped`.
2. **The refusal is invisible.** The player is on Friends or Watch, a menu is open, or the pet is an egg or Gone, so the toast is never drawn. Pinned in Task 3 by `a_tap_brings_the_game_forward` and `station_toasts_show_on_the_egg_and_gone_screens`, and in Task 4 by `every_refusal_raises_a_toast`.
3. **The opt-in unsigned path persists a refused station message as the badge's broadcast profile.** This is today's behaviour on a cooldown: `apply` returns `None`, so `handle_ndef_write` falls through to `SetProfile`. Pinned in Task 4 by `station_outcomes_are_transient_on_the_plain_path` (`Outcome::is_station`).
4. **The cooldown table can be farmed or locks out.** A ninth station evicts an entry that is still resting; `cd=` outside 5 to 1440; a refusal re-stamps and extends the wait. Pinned in Task 4 by `the_ninth_station_evicts_the_one_free_soonest`, `cd_is_clamped`, `a_refused_tap_does_not_extend_the_wait` and `two_stations_back_to_back`.
5. **The badge sleeps through a change the fix caused.** Examples: a mood bump, a cleared timer, `no_groggy`, the demo pause. Pinned in Task 2 by `a_fix_keeps_the_wake_exact` (random states and every preset, against `first_change_min`), and in Task 5 by the oracle-against-delta station runs.

Also pinned, outside the top five: old phrases with other capitals, spaces or a trailing newline (Task 1 vectors); a reader newer than the badge (`unknown names are skipped`, Task 1); the fallback alias exists only for the three presets with an exact old-phrase equivalent (Task 6 `fallback_only_for_exact_equivalents`).

## File map

| File | Change |
|---|---|
| `src/game/engine/fix.rs` | create: `Fix`, `Level`, `Target`, `Op`, `cond`, `Reject`, `normalize`, `ALIASES`, `expand_alias`, `parse`, `parse_text`, `safe_range`, `apply`, host-only `canonical` |
| `src/game/engine/mod.rs` | `pub mod fix;`; `GameState::station_fix`; remove `station_food`, `station_drugs`, `station_rest`, `STATION_FOOD_FLOOR_PCT`, `STATION_BOOZE_CAP_PCT` |
| `src/game/engine/thresholds.rs` | 4 fields, Classic and Casual, keys, setters, one `validate` rule |
| `assets/to-badge/BPETS_V3.CFG`, `USER_GAMES.md` | regenerated template; 4 keys in the key list |
| `src/game/engine/sim.rs` | `Act::Station`, `with_stations`, `stations_only` |
| `src/game/mod.rs` | `StationToast`, `station_lines`, `show_station`, `bring_game_forward`, `Toast::Station`; `draw_two_line_toast` (from `draw_cause_toast`); toasts on the Hatching and Gone screens; remove `Toast::Station{Food,Drugs,Inspire,Rest,Cooldown}`, `STATION_COOLDOWN_SECS`, `show_station_cooldown` |
| `src/game/battle_view.rs` | `open_pre_battle` calls `bring_game_forward` |
| `src/game/station.rs` | rewrite: `Cooldowns`, `cooldown_ticks`, `Outcome`, `apply(text, signed)` |
| `src/game/lifecycle.rs` | `game_phase`; remove `can_use_station` |
| `src/fw/nfct.rs` | both station call sites use `station::apply` and `Outcome::sw` / `is_station`; the screen switch at 418 goes |
| `tests/station_v3.rs` | create: `mod parse`, `mod apply`, `mod sim` (ignored long runs) |
| `tests/data/station_vectors.txt` | create: shared accept / reject / ignore vectors (Rust and Python) |
| `scripts/station_presets.txt` | create: the eight presets of 19.5 |
| `tests/engine_v3.rs` | `station_aliases` and the preview-sleep test go through `station_fix` |
| `scripts/station.py`, `scripts/test_station.py` | create: organiser tool and its tests |
| `.github/workflows/tests.yml` | matrix suite `station_v3` with `--include-ignored`; guide job runs `test_station.py` |
| `src/guide/data.rs`, `src/guide/shots.rs`, `guide/lib/social.typ` | stations JSON; two shots; preset and alias tables |
| `guide/chapters/11-friends-battles-stations.typ`, `guide/chapters/05-care-actions.typ` | Stations section rewritten; station toast rows |
| `NFC_README.md`, `USER_NFC.md`, `README.md`, `QUICKSTART.md`, `scripts/README.md` | new strings, presets, status words, the organiser tool |
| `../../android_nfc/BadgeCtl/...` | gated Task 8 [Q2], its own repo and commit |

---

### Task 1: the fix message (parser and aliases)

Spec 19.2 (grammar, 96 bytes, reject and skip rules), 19.3 (aliases), 19.5 (presets as data).

Flash: about +1.5 KiB (tokeniser, name and condition tables, four alias strings of about 170 bytes).

**Files:**
- Create: `src/game/engine/fix.rs`, `tests/station_v3.rs` (`mod parse`), `tests/data/station_vectors.txt`, `scripts/station_presets.txt`.
- Modify: `src/game/engine/mod.rs` (`pub mod fix;` only).

**Interfaces:**
- Consumes: nothing new.
- Produces:
```rust
// fix.rs
pub const FIX_MAX_LEN: usize = 96;      // spec 19.2, raised from 32
pub const STATION_ID_MAX: usize = 8;
pub const MAX_LEVELS: usize = 16;       // 96 bytes hold at most 14 level items
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target { Full, Water, Energy, Fun, Dirt, Booze, Strain, Fit, Weight, Health }
impl Target { pub const ALL: [Target; 10]; pub fn name(self) -> &'static str; } // "full" ... "health", spec order
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op { Set, AtLeast, AtMost, Add, Sub }  // '=' '>' '<' '+' '-'
impl Op { pub fn ch(self) -> char; }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Level { pub target: Target, pub op: Op, pub pct: u8 }   // pct 0..=100
pub mod cond {   // bits of Fix::clear, in canonical order
    pub const HANGOVER: u8 = 1; pub const INJURY: u8 = 2; pub const GROGGY: u8 = 4;
    pub const DEPRIVED: u8 = 8; pub const DEPENDENCE: u8 = 16; pub const DIABETES: u8 = 32;
    pub const CHRONIC: u8 = DEPENDENCE | DIABETES;
    pub const NAMES: [(&str, u8); 6]; // ("hangover", HANGOVER) ... ("diabetes", DIABETES)
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fix { id: [u8; STATION_ID_MAX], id_len: u8, pub levels: heapless::Vec<Level, MAX_LEVELS>, pub clear: u8, pub cd_min: Option<u16> }
impl Fix {
    /// `id` must be 1..=8 bytes of [a-z0-9] (callers: the parser, tests). Panics otherwise.
    pub fn new(id: &[u8]) -> Fix;
    pub fn id(&self) -> &[u8];
    /// Something the badge acts on: a level item or a known condition (`cd=` alone is not).
    pub fn understood(&self) -> bool;
    #[cfg(any(test, feature = "simulator"))]
    pub fn canonical(&self) -> heapless::String<FIX_MAX_LEN>;
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reject { Unreadable, BadId, BadItem, Over100, Duplicate, NothingKnown }
impl Reject { pub fn code(self) -> &'static str; } // "unreadable" "bad_id" "bad_item" "over_100" "duplicate" "nothing_known"
/// Trim ASCII whitespace, lowercase, turn every run of ASCII whitespace into one space.
/// None when the result is empty, holds a non-ASCII byte, or is longer than FIX_MAX_LEN.
pub fn normalize(text: &[u8]) -> Option<heapless::Vec<u8, FIX_MAX_LEN>>;
pub const ALIASES: [(&str, &str); 4] = [
    ("more food", "fix s=food full>60 water>60"),
    ("more drugs", "fix s=drugs health>100 booze<20 clear=hangover,injury,groggy"),
    ("sleep like a bear", "fix s=rest energy>100 clear=groggy,deprived"),
    ("more inspiration", "fix s=inspire fun>60"),   // [Q3]
];
/// The alias expansion of a normalised text, or the text itself.
pub fn expand_alias(normalized: &[u8]) -> &[u8];
/// Ok(None): not a station message (the first word is not "fix").
pub fn parse(normalized: &[u8]) -> Result<Option<Fix>, Reject>;
/// normalize, expand_alias, parse. A text normalize refuses is Err(Unreadable) when
/// its trimmed, lowercased start is "fix" followed by a space or the end, else Ok(None).
pub fn parse_text(text: &[u8]) -> Result<Option<Fix>, Reject>;
```
- **Parse rules** (spec 19.2, plus planner calls 1 to 4):
  - Split on single spaces. Token 0 must be `fix`, else `Ok(None)`.
  - Token 1 must be `s=` followed by 1 to 8 of `[a-z0-9]`. If it is missing, empty, too long or uses another character: `BadId`.
  - Every later token is `name op rest`. `op` is the first of `= > < + -` in the token. `name` is the bytes before it and must be 1 or more of `[a-z]`. `rest` must not be empty. Anything else: `BadItem`.
  - A `Target` name needs `rest` of 1 to 3 digits (leading zeros allowed), else `BadItem`. A value over 100 gives `Over100`.
  - `clear` needs op `=`. `rest` is a comma list of 1 or more `[a-z]` words, else `BadItem`. Known conditions set their bit; unknown ones are skipped. Several `clear=` tokens add up.
  - `cd` needs op `=` and 1 to 4 digits, else `BadItem`. A second `cd`: `Duplicate`.
  - A second `s`: `Duplicate`.
  - Any other name is skipped (a newer reader).
  - Same target twice: both are kept, in message order.
  - At the end, `!fix.understood()` gives `NothingKnown`.
- **Canonical form** (host only; the vectors and `scripts/station.py` print it): `fix s=<id>`, then ` <name><op><value>` per level in order (value without leading zeros), then ` clear=<a,b,...>` in `cond::NAMES` order when `clear != 0`, then ` cd=<n>` when present.
- `scripts/station_presets.txt`: `#` comment lines, then one `place|plaintext` line per preset, in the order of spec 19.5:
```text
Food stall|fix s=food1 full>60 water>60
Pharmacy|fix s=pharm1 health>100 booze<20 clear=hangover,injury,groggy
Nap tent|fix s=nap1 energy>100 clear=groggy,deprived
Spa|fix s=spa1 dirt<10 fun<60 strain<10 clear=groggy
Bar recovery|fix s=bar1 booze<10 water>60 clear=hangover
Water point|fix s=water1 water>70
Gym|fix s=gym1 fit+5 strain<20 cd=240
Clinic|fix s=clinic1 clear=dependence,diabetes booze<5 fit>50 cd=1440
```
- `tests/data/station_vectors.txt`: `#` comments, then tab-separated `accept<TAB>input<TAB>canonical`, `reject<TAB>input<TAB>code` or `ignore<TAB>input`. The input field is taken verbatim, spaces included.

- [ ] **Step 0: merge.** `git merge main` (plans 5, 6, 8 in). Re-find the anchors. If an assumption under "Before Task 1" no longer holds, stop and report.
- [ ] **Step 1: write the vectors file** with at least these lines (`⇥` is a tab):
```text
accept⇥fix s=food1 full>60 water>60⇥fix s=food1 full>60 water>60
accept⇥  FIX S=Food1  full>060   water>60 ⇥fix s=food1 full>60 water>60
accept⇥more food⇥fix s=food full>60 water>60
accept⇥More Drugs⇥fix s=drugs health>100 booze<20 clear=hangover,injury,groggy
accept⇥sleep like a bear⇥fix s=rest energy>100 clear=groggy,deprived
accept⇥more inspiration⇥fix s=inspire fun>60
accept⇥fix s=clinic1 clear=dependence,diabetes booze<5 fit>50 cd=1440⇥fix s=clinic1 booze<5 fit>50 clear=dependence,diabetes cd=1440
accept⇥fix s=x1 sparkle>5 full>50⇥fix s=x1 full>50
accept⇥fix s=x1 clear=hangover,glitter⇥fix s=x1 clear=hangover
accept⇥fix s=x1 clear=injury clear=hangover⇥fix s=x1 clear=hangover,injury
accept⇥fix s=x1 full>50 full<70⇥fix s=x1 full>50 full<70
accept⇥fix s=x1 fun>1 cd=0⇥fix s=x1 fun>1 cd=0
accept⇥fix s=x1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1⇥fix s=x1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1
reject⇥fix s=x1 full>101⇥over_100
reject⇥fix s=x1 full>1000⇥bad_item
reject⇥fix full>50⇥bad_id
reject⇥fix s=toolongid full>50⇥bad_id
reject⇥fix s= full>50⇥bad_id
reject⇥fix s=a_b full>50⇥bad_id
reject⇥fix⇥bad_id
reject⇥fix s=x1⇥nothing_known
reject⇥fix s=x1 sparkle>5⇥nothing_known
reject⇥fix s=x1 cd=60⇥nothing_known
reject⇥fix s=x1 full>⇥bad_item
reject⇥fix s=x1 full>5a⇥bad_item
reject⇥fix s=x1 full⇥bad_item
reject⇥fix s=x1 >5⇥bad_item
reject⇥fix s=x1 clear>hangover⇥bad_item
reject⇥fix s=x1 clear=⇥bad_item
reject⇥fix s=x1 clear=hangover,,injury⇥bad_item
reject⇥fix s=x1 cd=12345 fun>1⇥bad_item
reject⇥fix s=x1 cd<5 fun>1⇥bad_item
reject⇥fix s=x1 fun>1 cd=5 cd=6⇥duplicate
reject⇥fix s=x1 s=x2 fun>1⇥duplicate
reject⇥fix s=x1 fün>1⇥unreadable
reject⇥fix s=x1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1 fun>1⇥unreadable
ignore⇥hello
ignore⇥https://badge.team
ignore⇥fixme s=x1 fun>1
ignore⇥
```
The 14-item `accept` line is 92 bytes; the 15-item `reject` line is 98 bytes.
- [ ] **Step 2: write the failing tests** in `tests/station_v3.rs`, `mod parse`:
```rust
#[test] fn the_shared_vectors_hold() {
    // include_str!("data/station_vectors.txt"); for each line: accept -> parse_text(input) == Ok(Some(f)) and
    // f.canonical() == canonical; reject -> Err(r) with r.code() == code; ignore -> Ok(None). Message names the line.
}
#[test] fn every_preset_parses_and_is_understood() {
    // include_str!("../scripts/station_presets.txt"): 8 lines; parse_text(plaintext) is Ok(Some(f)), f.understood(),
    // f.canonical() == plaintext for all but the Clinic (its canonical moves clear= after the levels)
}
#[test] fn every_alias_is_a_valid_fix() { /* for (phrase, fix) in ALIASES: parse_text(phrase) == parse_text(fix), both Ok(Some) */ }
#[test] fn normalize_keeps_the_limit() {
    // normalize(b"") == None; normalize(b" \t\n") == None; 96 bytes Some; 97 bytes None; b"A\tB" -> b"a b"
}
#[test] fn the_id_is_kept() { /* parse_text(b"fix s=Gym1 fit+5").unwrap().unwrap().id() == b"gym1" */ }
```
- [ ] **Step 3: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --test station_v3 parse::` fails (`fix` missing).
- [ ] **Step 4: implement** `fix.rs` as above, plus `pub mod fix;`. Engine rules apply: no `alloc`, no float.
- [ ] **Step 5: run them and see them pass.** The same command: PASS.
- [ ] **Step 6: commit** `feat(bornpets): fix message parser with the old phrases as aliases`.
- [ ] **Step 7: V.**

### Task 2: applying a fix, and its tunables

Spec 19.2 ("What the badge does": clamps, health ceiling, clinic, the 60 % fit cap, several needs at one event, +5 mood) and 19.4 (delta engine, demo, hibernation, asleep, hard mode, oversleep).

Flash: about +1.0 KiB new, about −0.3 KiB for the three old station functions; net about +0.7 KiB.

**Files:**
- Modify: `src/game/engine/fix.rs` (`safe_range`, `apply`), `src/game/engine/mod.rs` (`station_fix`; remove `station_food`, `station_drugs`, `station_rest`, `STATION_FOOD_FLOOR_PCT`, `STATION_BOOZE_CAP_PCT`), `src/game/engine/thresholds.rs`, `assets/to-badge/BPETS_V3.CFG`, `USER_GAMES.md` (key list), `src/game/station.rs` (the three closures call `station_fix` with the alias, so the crate keeps building; Task 4 rewrites the file), `src/guide/data.rs` (drop the rows `STATION_FOOD_FLOOR_PCT` and `STATION_BOOZE_CAP_PCT`; `make guide` is fixed in Task 7), `tests/engine_v3.rs` (`mod station_aliases`, the preview-sleep test).
- Test: `tests/station_v3.rs` `mod apply`.

**Interfaces:**
- Consumes: Task 1 `Fix`, `Level`, `Target`, `Op`, `cond`, `parse_text`.
- Produces:
```rust
// fix.rs
/// The levels a station may move `t` between, [lo, hi] (level units). The edges are the pet's health-damage lines,
/// so landing on an edge is safe (regime::health_damage uses strict comparisons):
/// Full, Water: [lvl(DMG_NEED_LOW_PCT), lvl(DMG_NEED_HIGH_PCT)]   (15, 95)
/// Energy: [lvl(DMG_ENERGY_PCT), MAX]                              (10, 100)
/// Fun: [0, MAX]
/// Dirt: [0, lvl(DMG_DIRT_PCT)]                                     (0, 85)
/// Booze: [0, needs::booze_danger()]                                (0, 70; Casual 75)
/// Strain: [0, needs::strain_inj()]                                 (0, 60; Casual 65)
/// Fit: [0, lvl(STATION_FIT_CAP_PCT)]                               (0, 60)
/// Weight: [lvl(DMG_WEIGHT_LOW_PCT), lvl(DMG_WEIGHT_HIGH_PCT)]     (20, 80)
/// Health: [lvl(needs::HEALTH_DANGER_PCT), regime::health_ceiling(s)]  (30, the pet's ceiling)
pub fn safe_range(s: &GameState, t: Target) -> (i32, i32);
/// Apply now: conditions first, then levels in message order, then the mood bump. No catch-up, no wake, no causes.
pub fn apply(s: &mut GameState, fix: &Fix);
// mod.rs
impl GameState {
    /// A station fix (spec 19.4): fix::apply, then player_acted() (the demo pause and note).
    /// Returns true when the demo pet thanks the player: demo on, awake, not hibernating.
    pub fn station_fix(&mut self, fix: &fix::Fix) -> bool;
}
```
- **A level item**, with `cur` the need's level, `(lo, hi) = safe_range(s, target)` and `v = if pct == 100 { MAX } else { pct * P }`:
```rust
let want = match op { Set => v, AtLeast => cur.max(v), AtMost => cur.min(v), Add => cur + v, Sub => cur - v };
// The clamp limits how far the item moves the need, never the other way (planner call 5):
let new = if want > cur { want.min(hi.max(cur)) } else { want.max(lo.min(cur)) };
```
  `Health` reads `health_ceiling` at the moment the item is applied, after the conditions are cleared.
- **Conditions:**
  - `HANGOVER`: `hangover_min = 0`.
  - `INJURY`: `injured_min = 0`.
  - `GROGGY`: `groggy_min = 0`.
  - `DEPRIVED`: `deprived = false`.
  - `DEPENDENCE`, as a finished Rehab course (`advance.rs` 183): `dependent = false`, `dependence_min = 0`, `rehab_min = 0`, `recovering_min = RECOVERING_MIN()`.
  - `DIABETES`, as a reversal (`advance.rs` 156): `diabetic = false`, `diabetes_min = 0`, `reversal_min = 0`.
- **Oversleep (19.4):** `no_groggy = true` when the fix has an `Energy` item with op `Set`, `AtLeast` or `Add`.
- **Mood:** `mood += lvl(STATION_MOOD_PCT)`, saturating at `MAX`.
- Nothing else changes: no money, no cooldown field, no wake-up (`is_sleeping` stays), no hibernation change, no cause. `hard_mode` is never read: a fix is free.
- Old behaviour removed: `station_food`, `station_drugs`, `station_rest` and the two consts. The aliases reproduce them exactly (spec 19.3), plus the +5 mood.

**Tunables** (Classic / Casual; one block `// NFC stations (plan 4)` after the last field of each list):

| Key | Classic | Casual | Setter | Meaning |
|---|---|---|---|---|
| `STATION_MOOD_PCT` | 5 | 5 | `v.min(100) as u8` | mood bump of every accepted fix |
| `STATION_FIT_CAP_PCT` | 60 | 60 | `clamp_pct(v)` | a station never raises fitness past this |
| `STATION_CD_MIN` | 5 | 5 | `clamp_u16(v).clamp(1, CD_LIMIT_MIN)` | default cooldown per station, and the shortest a `cd=` can ask for |
| `STATION_CD_MAX_MIN` | 1440 | 1440 | `clamp_u16(v).clamp(1, CD_LIMIT_MIN)` | the longest `cd=` (one day) |

`const CD_LIMIT_MIN: u16 = super::demo::MINUTES_PER_DAY;`. `validate`: `STATION_CD_MIN > STATION_CD_MAX_MIN` sets both to the preset. Casual equals Classic (planner call 7).

- [ ] **Step 1: write the failing tests** in `tests/station_v3.rs` `mod apply`. Helpers: `pet()` as in `engine_v3::station_aliases`, `pct(p)`, and `fix_of(t: &[u8]) -> Fix` (= `parse_text(t).unwrap().unwrap()`).
```rust
#[test] fn the_edges_are_the_damage_lines() {
    // on pet(): full=100 -> 95 %; water=100 -> 95 %; energy=0 -> 10 %; dirt=100 -> 85 %; booze=100 -> BOOZE_DANGER_PCT;
    // strain+100 -> STRAIN_INJ_PCT; weight=0 -> 20 %; weight=100 -> 80 %; health=0 -> 30 %; health>100 -> health_ceiling;
    // fit=90 from 30 % -> 60 %; fun=0 -> 0
}
#[test] fn fit_is_capped_but_never_lowered() {
    // fit+5: 58 % -> 60 %, 70 % -> 70 %; fit>50: 30 % -> 50 %, 80 % -> 80 % (the clinic, 19.2)
}
#[test] fn a_pet_outside_the_range_is_only_helped() {
    // full>60 on 98 %: stays 98 %; dirt<10 on 95 %: 10 %; booze<20 on 90 %: 20 %; weight<70 on 90 %: 70 %
}
#[test] fn the_clamp_never_works_against_the_item() {
    // for every Target and Op and pct in {0, 1, 50, 99, 100} and start in {0, 10, 50, 90, 100} %:
    // AtLeast/Add never lower the need, AtMost/Sub never raise it, Set lands between start and v
}
#[test] fn no_fix_raises_health_damage() {
    // 3000 random states (fields as status_wake::wake_is_exact_on_random_states, LCG seed 1) x a random Fix
    // (0..=5 levels, random target, op, pct 0..=100; random clear bits): with `probe` = the after-state with its health
    // set back to the before value, health_damage(&probe) <= health_damage(&before). (Health itself only gates the
    // sleep-shortage term: a healed pet can be damaged by low energy again, which is not a station's doing.)
    // mood after >= mood before; money, phase, is_sleeping, hibernating unchanged; take_causes() is empty
}
#[test] fn conditions_clear_and_chronic_ones_like_their_cures() {
    // clinic preset on a dependent, diabetic pet: !dependent, dependence_min 0, rehab_min 0,
    // recovering_min == RECOVERING_MIN(); !diabetic, diabetes_min 0, reversal_min 0
}
#[test] fn health_reads_the_ceiling_after_the_clears() {
    // untreated diabetic, fit 80 %: fix "fix s=t health>100 clear=diabetes" -> health == ceiling of the cured pet
}
#[test] fn an_energy_fix_forgives_the_next_oversleep() { /* energy>100 and energy+5 set no_groggy; energy<50 does not */ }
#[test] fn every_fix_lifts_mood_by_the_bump() { /* mood 50 % -> 50 % + lvl(STATION_MOOD_PCT); mood MAX stays MAX */ }
#[test] fn asleep_and_hibernating_pets_stay_so() {
    // is_sleeping stays true after the nap-tent preset; hibernating pet: values changed, still hibernating,
    // update(t + 600) leaves them unchanged (frozen)
}
#[test] fn the_demo_pet_pauses_and_thanks() {
    // demo.on, awake: station_fix(..) == true, demo.pause_min == DEMO_PAUSE_MIN(), note None;
    // asleep: false but the pause is set; demo off: false, pause_min unchanged
}
#[test] fn hard_mode_pays_nothing() { /* hard_mode, money 50: after the pharmacy preset money == 50 and every plan 6 debt or bill field unchanged */ }
#[test] fn several_needs_at_one_event() { /* the spa preset changes dirt, fun, strain, groggy in one call; last_update_tick unchanged */ }
#[test] fn a_fix_keeps_the_wake_exact() {
    // for each of the 8 presets and each of 300 random states (generator above): station_fix, then
    // expect = first_change_min(&s).map_or(t0 + 120 * 6, |m| t0 + m * 6) (copy first_change_min from engine_v3::status_wake);
    // assert_eq!(s.next_wake_tick(), expect)
}
#[test] fn station_tunables_have_keys_and_validate() {
    // all four keys in BORNPETS_CFG_KEYS; STATION_CD_MIN=10 with STATION_CD_MAX_MIN=5 -> validate gives 5 / 1440
}
```
- [ ] **Step 2: migrate the old tests.** In `tests/engine_v3.rs` `mod station_aliases` and `preview_sleep_is_the_night_slept_until_rested`, replace `s.station_food()`, `s.station_drugs()` and `s.station_rest()` with `s.station_fix(&fix_of(b"more food"))`, `(b"more drugs")` and `(b"sleep like a bear")`. Keep every assertion: they are the proof that the aliases do what the old functions did.
- [ ] **Step 3: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --test station_v3 apply::` (no `station_fix`).
- [ ] **Step 4: implement** `safe_range`, `apply`, `station_fix` and the tunables. Remove the old functions and consts and their rows in `src/guide/data.rs`. In `src/game/station.rs`, the three closures call `s.station_fix(&fix::parse_text(<phrase>).unwrap().unwrap())` for now. Regenerate the template: `BPETS_REGEN=1 CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --test engine_v3 template_`. Add the four keys to the `USER_GAMES.md` key list, one line each with both values.
- [ ] **Step 5: run them and see them pass.** `... --test station_v3 apply::`, `... --test engine_v3 station_aliases::`, `... --test engine_v3 preview_sleep`, `... --test engine_v3 template_`: PASS.
- [ ] **Step 6: commit** `feat(bornpets): station fix clamped to the damage lines, with its tunables`.
- [ ] **Step 7: V.**

### Task 3: station toasts and the game screen in front

Owner rule: a refusal tells the player why. Spec 19.4 (hard-mode toast, demo thanks). Plan 5 overlap: the same screen switch as a battle challenge.

Flash: about +0.5 KiB (five two-line texts, one draw path shared with the cause toast).

**Files:**
- Modify: `src/game/mod.rs`, `src/game/battle_view.rs` (`open_pre_battle` only).
- Test: `src/game/mod.rs` tests.

**Interfaces:**
- Consumes: `doses::fmt_wait`, `Reason`, `show_reason`, `friends_view::close`, `diary_view::close`, `traits_view::close`, `crate::DISPLAY_STATE`, `crate::SCREEN_GAME`, `crate::sync::SharedCell`.
- Produces:
```rust
// game/mod.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StationToast { Fixed, Thanks, Covered, TooSoon(u32), Unknown }   // TooSoon: seconds left
/// The two lines (Toast geometry table). `id` fills "at <id>"; TooSoon formats with doses::fmt_wait.
pub fn station_lines(t: StationToast, id: &[u8]) -> (&'static str, heapless::String<24>);
/// Store (t, id) in a SharedCell and show Toast::Station (show_toast: wakes the display loop).
pub fn show_station(t: StationToast, id: &[u8]);
/// Put the game screen in front, closing the views that would hide its toasts: Friends, Diary, Traits.
pub fn bring_game_forward();
#[cfg(test)] pub(crate) fn last_station_toast_for_test() -> Option<(StationToast, heapless::Vec<u8, 8>)>;
// Toast::Station = 9; from_u8(9) => Station; message() is "" (drawn as two lines)
// screen_words() gains ("toasts", ..) for: "Magic fix!", "at <station>", "thanks!", "Covered by",
// "magic, not insurance", "Too soon!", "again in <time>", "Unknown magic", "badge can't read it"
```
- **Drawing:** rename `draw_cause_toast` to `draw_two_line_toast(display, what: &str, why: &str)`. Its body does not change. The `Toast::Cause` branch in `draw_toast` calls it with `cause.what()` / `cause.why()`, and the new `Toast::Station` branch calls it with `station_lines(..)`.
- **Egg and Gone screens:** call `draw_toast(display)?` in the Hatching branch (before its `return`) and in the Gone branch (after the prompt lines, before its `return`). This lets a fix on an egg, and the `pet is gone` refusal, show (planner calls 11 and 12). `next_ui_deadline_tick` needs no change: it already clears a toast in any phase.
- `battle_view::open_pre_battle`: its three `close()` calls and the screen switch become one `super::bring_game_forward()`. The role check and the state stores stay.
- Leave the old `Toast::StationFood`..`StationCooldown` variants alone in this task: `station.rs` still uses them until Task 4.

- [ ] **Step 1: write the failing tests** (`src/game/mod.rs` tests):
```rust
#[test] fn station_lines_say_what_happened() {
    // Fixed + b"food1" -> ("Magic fix!", "at food1"); Thanks -> ("Magic fix!", "thanks!");
    // Covered -> ("Covered by", "magic, not insurance"); TooSoon(290) -> ("Too soon!", "again in 4m");
    // TooSoon(14_340) -> "again in 3h59"; TooSoon(86_400) -> "again in 24h00"; Unknown -> ("Unknown magic", "badge can't read it")
}
#[test] fn station_toasts_clear_the_hax_balance() {
    // as the_cause_toast_clears_the_hax_balance, for every StationToast with id b"abcdefgh" and TooSoon(86_400):
    // line 1 box meets no part of the "999999 H" box; line 2 box ends at x <= 150; line 1 <= 13 columns, line 2 <= 21
}
#[test] fn the_station_toast_sits_on_white() { /* as the_cause_toast_lines_sit_on_white_over_a_sprite, for Fixed b"spa1" */ }
#[test] fn station_toasts_show_on_the_egg_and_gone_screens() {
    // lock_test_state; hatching pet + show_station(Fixed, b"food1"): draw_screen_game has RED pixels in y 36..=62;
    // Gone pet + show_reason(Reason::Gone): RED pixels in y 36..=48; with no toast: none in y 36..=48 on either screen
}
#[test] fn a_tap_brings_the_game_forward() {
    // friends_view open, diary_view open, active screen SCREEN_MAIN: bring_game_forward() -> neither view active,
    // DISPLAY_STATE active_screen == SCREEN_GAME
}
#[test] fn screen_words_list_the_station_toasts() { /* all nine texts above are in screen_words() under "toasts" */ }
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --lib -- game::tests::`.
- [ ] **Step 3: implement** as above.
- [ ] **Step 4: run them and see them pass.** The same command, then `... --lib -- game::battle_view::` and `... --test battle_v3` (non-ignored only): PASS.
- [ ] **Step 5: commit** `feat(game): two-line station toasts, shown on every pet screen`.
- [ ] **Step 6: V.**

### Task 4: the station dispatcher, cooldowns and status words

Spec 19.2 (cooldown per station id, 8 ids, 5-minute default, `cd=` up to 1440, never under 5; reject codes), 19.4 (a fix is an event), 19.7 (`station.rs` and `nfct.rs`).

Flash: about +0.7 KiB new, about −0.3 KiB old (three atomics, the old toasts and the cooldown formatter); net about +0.4 KiB.

**Files:**
- Modify: `src/game/station.rs` (rewrite), `src/game/mod.rs` (remove `Toast::StationFood`, `StationDrugs`, `StationInspire`, `StationRest`, `StationCooldown` with their `from_u8` and `message` arms, `STATION_COOLDOWN_SECS`, `show_station_cooldown` and the cooldown branch of `draw_toast`; discriminants 10 to 13 become unused), `src/game/lifecycle.rs` (`game_phase`; remove `can_use_station`, and its assert at 1966 becomes `assert_eq!(game_phase(), Some(Phase::Gone))`), `src/fw/nfct.rs`, `src/guide/data.rs` (drop the `STATION_COOLDOWN` row: `COOLDOWN_TICKS` goes; Task 7 fixes the guide text).
- Test: `src/game/station.rs` tests.

**Interfaces:**
- Consumes: Task 1 `fix::parse_text`, `Fix::id`, `cond::CHRONIC`; Task 2 `GameState::station_fix`, `STATION_CD_MIN`, `STATION_CD_MAX_MIN`; Task 3 `show_station`, `StationToast`, `bring_game_forward`; `show_reason(Reason::Gone)`; `lifecycle::{now_tick, with_game_now}`; `modal::{is_open, close}`.
- Produces:
```rust
// station.rs
pub const COOLDOWN_SLOTS: usize = 8;
#[derive(Clone, Copy)]
struct Slot { id: [u8; STATION_ID_MAX], len: u8, until: u32 }   // until: tick the station is free again
pub struct Cooldowns { slots: [Option<Slot>; COOLDOWN_SLOTS] }
impl Cooldowns {
    pub const fn new() -> Self;
    /// Ticks left for `id`, None when free.
    pub fn remaining(&self, id: &[u8], now: u32) -> Option<u32>;
    /// Remember `id` until now + ticks: its own slot, else an empty or free slot, else the slot that frees soonest.
    pub fn stamp(&mut self, id: &[u8], now: u32, ticks: u32);
}
/// clamp(cd.unwrap_or(STATION_CD_MIN()), STATION_CD_MIN(), STATION_CD_MAX_MIN()) * TICKS_PER_MINUTE
pub fn cooldown_ticks(cd_min: Option<u16>) -> u32;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome { Applied, Refused, Unknown, NotStation }
impl Outcome {
    pub fn sw(self) -> [u8; 2];          // status-word table [Q1]
    /// The text was a station message: the plain NDEF path keeps it transient (anything but NotStation).
    pub fn is_station(self) -> bool;
}
/// One tap. `signed`: it came through the verified signed channel.
pub fn apply(text: &[u8], signed: bool) -> Outcome;
#[cfg(test)] pub(crate) fn reset_for_test();
// static COOLDOWNS: crate::sync::SharedCell<Cooldowns> = SharedCell::new(Cooldowns::new());
// lifecycle.rs
pub fn game_phase() -> Option<Phase>;    // None: no game started
```
- **`apply`**, in this order:
  1. `parse_text(text)`.
     - `Ok(None)`: when `signed`, the result is `Unknown` (toast `Unknown`, game brought forward). When not signed it is `NotStation`: no toast, no screen change.
     - `Err(_)`: `Unknown` (toast, game brought forward) on both paths.
  2. When not `signed`, clear `cond::CHRONIC` from `fix.clear` (planner call 9). If the fix is then not understood: `Unknown`, as above.
  3. `game_phase()`.
     - `None`: `Refused`. Bring the game forward (the start screen); no toast.
     - `Some(Gone)`: `Refused`, `show_reason(Reason::Gone)`, game forward.
     - `Hatching`, `Active` and `Leaving` go on (planner call 11).
  4. `COOLDOWNS.remaining(id, now_tick())` is `Some(t)`: `Refused`, `show_station(TooSoon(t * 10), id)`, game forward. Nothing is stamped.
  5. `with_game_now(|s| { thanks = s.station_fix(&fix); hard = s.hard_mode; s.request_save(); })`. Then stamp `cooldown_ticks(fix.cd_min)`. Show `Covered` if `hard`, else `Thanks` if `thanks`, else `Fixed`, with the id. Then the game is brought forward and an open menu is closed (`modal::close()`). Result `Applied`.
  - "Game forward" is always `bring_game_forward()` followed by `modal::close()` when `modal::is_open()`.
- **`nfct.rs`** (firmware only):
  - In `handle_signed`, the `#[cfg(feature = "game")]` block becomes:
```rust
let outcome = crate::game::station::apply(plaintext, true);
info!("signed: station {}", outcome);
let _ = out.extend_from_slice(&outcome.sw());
```
    The `DISPLAY_STATE` switch at 418 goes. `Outcome` derives `defmt::Format` under `embassy-core`.
  - In `handle_ndef_write`, the opt-in block becomes `if crate::game::station::apply(text, false).is_station() { return WriteOutcome::Transient; }`. The `show_toast` call goes: `apply` raises its own toasts.

- [ ] **Step 1: write the failing tests** (`station.rs` tests; each takes `lock_test_state()`, calls `reset_for_test()` and sets the time with `platform::set_fake_uptime_ms`):
```rust
#[test] fn an_alias_applies_and_says_where() {
    // active pet, fullness 20 %: apply(b"more food", true) == Applied; fullness >= 60 %;
    // last_station_toast_for_test() == (Fixed, b"food"); active screen SCREEN_GAME
}
#[test] fn the_same_station_rests_and_says_how_long() {
    // apply food1 -> Applied; again at once -> Refused, toast TooSoon(300); 60 s later -> TooSoon(240); 300 s later -> Applied
}
#[test] fn two_stations_back_to_back() { /* food1 then pharm1 at the same tick: both Applied */ }
#[test] fn a_refused_tap_does_not_extend_the_wait() { /* food1 at 0, again at 100 s (Refused), Applied at 300 s */ }
#[test] fn cd_is_clamped() {
    // cooldown_ticks(None) == 30; Some(0) == 30; Some(240) == 1440; Some(9999) == 8640;
    // gym preset: second tap 3 h later -> TooSoon(3600)
}
#[test] fn the_ninth_station_evicts_the_one_free_soonest() {
    // stamp ids s1..s8 with cooldowns 10..=17 min at tick 0; stamp s9: s1 forgotten (remaining None), s2..s9 kept;
    // with s4 already free at stamp time, s4's slot is taken instead and s1 kept
}
#[test] fn unknown_text_on_each_path() {
    // apply(b"hello", true) == Unknown, toast Unknown; apply(b"hello", false) == NotStation, no toast, screen unchanged;
    // apply(b"fix s=x full>101", false) == Unknown, toast Unknown; pet unchanged in all three
}
#[test] fn every_refusal_raises_a_toast() {
    // Gone pet: Refused + Reason::Gone shown; cooldown: TooSoon; unknown: Unknown; no game (set_state_for_test(None)):
    // Refused, no toast, active screen SCREEN_GAME
}
#[test] fn an_egg_takes_the_fix() { /* hatching pet: apply food1 == Applied, fullness >= 60 % */ }
#[test] fn the_plain_path_cannot_cure() {
    // dependent, diabetic pet: apply(clinic, false) -> Applied (booze<5 and fit>50 are understood), still dependent and diabetic;
    // apply(b"fix s=c2 clear=diabetes", false) == Unknown; apply(clinic, true) clears both
}
#[test] fn hard_mode_is_covered_and_demo_says_thanks() { /* hard_mode -> toast Covered; demo on, awake -> Thanks; neither -> Fixed */ }
#[test] fn a_tap_closes_the_menu() { /* modal::open(Stats); apply food1; !modal::is_open() */ }
#[test] fn outcome_status_words() { /* Applied [0x90,0], Refused [0x69,0x85], Unknown and NotStation [0x6A,0x82] */ }
#[test] fn station_outcomes_are_transient_on_the_plain_path() {
    // is_station(): Applied, Refused, Unknown true; NotStation false. A cooldown refusal on the plain path is Refused.
}
```
- [ ] **Step 2: run and see them fail.** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --lib -- game::station:: game::lifecycle::`.
- [ ] **Step 3: implement** `station.rs`, `game_phase`, the removals and the two `nfct.rs` call sites.
- [ ] **Step 4: run them and see them pass.** The same command, plus `... --lib -- game::tests::` and `... --test station_v3`: PASS. Then the three firmware `cargo check` commands of Global Constraints (`embassy-1680-full`, `embassy-game`, `embassy-1680-full,nfc-plaintext-station`): no errors, no new warnings.
- [ ] **Step 5: commit** `feat(game): station dispatcher with per-station cooldowns and status words`.
- [ ] **Step 6: V.**

### Task 5: stations in the simulator (long runs, CI only)

Spec 19.6 (a festival player visits the three alias stations once a day at 15:00), 13.5 (delta engine against the step oracle).

Flash: 0 (host only).

**Files:**
- Modify: `src/game/engine/sim.rs`, `.github/workflows/tests.yml`.
- Test: `tests/station_v3.rs` `mod sim`.

**Interfaces:**
- Consumes: Task 2 `station_fix`; `fix::parse_text`; `sim::{run_with, balanced, party, once_a_day, absent, Setup, Driver, Outcome, Plan}`.
- Produces:
```rust
// sim.rs
pub enum Act { /* existing */ Station(&'static [u8]) }   // perform: s.station_fix(&fix::parse_text(t).unwrap().unwrap()); true
/// The three alias stations once a day at 15:00 (19.6), after `base`'s own plan for that minute (the plan holds at most 8 steps).
pub fn with_stations(base: Routine<'_>) -> impl Fn(u32, u32, &GameState) -> Plan + '_;
/// Only the stations, nothing else.
pub fn stations_only(d: u32, m: u32, s: &GameState) -> Plan;
```
  `Routine` becomes `pub type Routine<'a>`. If `base` already used all 8 steps at 15:00, push the stations at 15:01 instead.

- [ ] **Step 1: write the tests**, each `#[ignore]`, 30 days, `Driver::Oracle` and `Driver::Delta`, seed 1, `Setup { clock: true, ..Default::default() }`:
```rust
#[test] #[ignore] fn balanced_with_stations_thrives() { /* both drivers: alive, avg_mood_pct >= 90, end_health_pct >= 95 */ }
#[test] #[ignore] fn party_with_stations_survives() { /* both: alive, end_health_pct >= 55, !diabetic (19.6: 65, not diabetic) */ }
#[test] #[ignore] fn stations_alone_do_not_replace_care() { /* both: !alive, days in 5.0..=12.0 (19.6: 8.6) */ }
#[test] #[ignore] fn once_a_day_lives_longer_with_stations() { /* days with stations > days without + 5.0 (19.6: 15.6 against 2.0) */ }
#[test] #[ignore] fn delta_matches_the_oracle_with_stations() {
    // for the four routines above: alive equal; avg_mood_pct, end_health_pct, end_fitness_pct, end_weight_pct within 2;
    // eprintln! every Outcome (CI log) for the report
}
```
- [ ] **Step 2: CI.** In `tests.yml`, add `station_v3` to the matrix suite list and to the `others` exclusion list, and an arm `station_v3) cargo test --features simulator --test station_v3 -- --include-ignored --nocapture ;;` next to `battle_v3`. Every job keeps its `if: github.server_url == 'https://github.com'`.
- [ ] **Step 3: run locally without the ignored ones.** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --test station_v3`: PASS, and the five show as ignored. Do not run them locally: they run in Task 9 on CI. If Task 9 shows a bound missed, Task 11 reports the numbers to the owner instead of moving the bound.
- [ ] **Step 4: commit** `test(bornpets): stations in the simulator, delta against oracle`.
- [ ] **Step 5: V.**

### Task 6: the organiser tool and the NFC docs

Spec 19.5 (presets) and 19.7 ("`NFC_README.md` and the reader tool get the new strings"); [Q1] fallback; [Q2] the in-repo tool.

Flash: 0.

**Files:**
- Create: `scripts/station.py`, `scripts/test_station.py`.
- Modify: `.github/workflows/tests.yml` (guide job: `- run: python3 -m unittest discover -s scripts -p 'test_station.py'` after the `test_copy_assets.py` step), `NFC_README.md`, `USER_NFC.md`, `README.md` (272 to 285), `QUICKSTART.md` (183 to 190), `scripts/README.md` (one section).

**Interfaces:**
- Consumes: `tests/data/station_vectors.txt`, `scripts/station_presets.txt` (Task 1); the status words (Task 4).
- Produces (`scripts/station.py`, Python 3 stdlib; `cryptography` imported only by `build_apdu`, `pyscard` only by `send`):
```python
FIX_MAX_LEN = 96
ALIASES = {"more food": "fix s=food full>60 water>60", ...}   # the same four as fix::ALIASES [Q3]
class Reject(Exception): code: str                           # the codes of Reject::code
def normalize(text: str) -> str | None: ...
def parse_text(text: str) -> dict | None: ...                # {"id", "levels": [(name, op, value)], "clear": [names], "cd": int|None}; raises Reject
def canonical(fix: dict) -> str: ...                         # same form as Fix::canonical
def load_presets(path: str = PRESETS) -> list[tuple[str, str]]: ...
def fallback_for(plaintext: str) -> str | None: ...          # the alias whose fix has the same canonical items (ignoring s=), else None
def build_apdu(seed: bytes, challenge: bytes, plaintext: bytes) -> bytes: ...   # 80 01 00 00 Lc plaintext||sig 00
def send(conn, seed: bytes, plaintext: bytes) -> int: ...     # GET CHALLENGE, SIGNED CMD; returns the SW as an int
# CLI:
#   station.py check TEXT...             canonical form and skipped items per text; exit 1 if any is rejected
#   station.py presets                   the preset table with each fallback alias
#   station.py apdu --key F --challenge HEX TEXT   the SIGNED CMD bytes in hex
#   station.py send --key F (--preset PLACE | --text TEXT) [--reader N] [--once]
#        wait for a badge, send, print "applied", "refused (cooldown, gone or no pet)" or "not understood";
#        on 6A 82 with a fallback alias, send the alias with a fresh challenge and say so [Q1]; wait for the badge to leave; repeat
```
- The Python parser follows the Task 1 rules exactly. The shared vectors file is the contract between the two.

- [ ] **Step 1: write the failing tests** in `scripts/test_station.py` (`unittest`):
```python
def test_shared_vectors(self): ...            # every line of tests/data/station_vectors.txt, as the Rust test does
def test_presets_parse(self): ...             # 8 presets, each accepted
def test_fallback_only_for_exact_equivalents(self): ...
    # food1 -> "more food", pharm1 -> "more drugs", nap1 -> "sleep like a bear"; spa1, bar1, water1, gym1, clinic1 -> None
@unittest.skipUnless(HAVE_CRYPTO, "cryptography not installed")
def test_apdu_layout_and_signature(self): ...
    # NFC_README §8 test seed, challenge bytes(range(16)), b"fix s=food1 full>60 water>60":
    # apdu[:4] == 80 01 00 00, apdu[4] == len + 64, plaintext follows, last byte 00, signature verifies over challenge||plaintext
def test_check_exit_codes(self): ...          # main(["check", "more food"]) == 0; main(["check", "fix s=x full>101"]) == 1
```
- [ ] **Step 2: run and see them fail.** `python3 -m unittest discover -s scripts -p 'test_station.py'`.
- [ ] **Step 3: implement** `station.py`.
- [ ] **Step 4: run and see them pass.** The same command: PASS (the crypto test may show as skipped locally).
- [ ] **Step 5: docs.**
  - `NFC_README.md` §4 is rewritten:
    - the grammar of 19.2;
    - what the badge does (clamps, mood, cooldown per id);
    - the alias table [Q3] and the preset table;
    - the new status-word table with `69 85` and the reader rule: fall back to the alias only on `6A 82`, never on `69 85` [Q1];
    - `scripts/station.py check` to test a string before signing it.
  - Also in `NFC_README.md`: the §2 status-word table gets the `69 85` row; §5.1 says a station message on the opt-in path is always transient, refused ones included; §10 lists `scripts/station.py`, `scripts/station_presets.txt` and `src/game/engine/fix.rs`.
  - `USER_NFC.md` replaces the v1 command table with the magic places and the toasts.
  - `README.md` drops "(feed, heal, inspire, sleep)".
  - `QUICKSTART.md`'s "A station tap did nothing" entry: a tap always shows a toast now, and the entry lists the three refusals.
  - `scripts/README.md`: a short "Station reader (organisers)" section with the four subcommands.
- [ ] **Step 6: commit** `feat(scripts): station checker and sender; NFC docs for the fix message`.
- [ ] **Step 7: V.**

### Task 7: the player's guide

Roadmap row 4: updates guide ch 11. Every marked word explained; `make guide` green.

Flash: 0.

**Files:**
- Modify: `src/guide/data.rs`, `src/guide/shots.rs`, `guide/lib/social.typ`, `guide/chapters/11-friends-battles-stations.typ` (Stations section), `guide/chapters/05-care-actions.typ` (toast table and the two station paragraphs), `tests/guide_shots.rs` only if a shot test needs a new name.

**Interfaces:**
- Consumes: `fix::ALIASES`, `scripts/station_presets.txt` (`include_str!("../../scripts/station_presets.txt")`), Task 2 keys, Task 4 `station::apply`, `COOLDOWN_SLOTS`.
- Produces:
```rust
// data.rs fixed() (STATION_COOLDOWN is gone since Task 4): add
//   ("STATION_COOLDOWN_SLOTS", COOLDOWN_SLOTS as u32, Unit::Count)
// dump(): a "stations" object: {"aliases": [{"phrase", "fix"}...], "presets": [{"place", "fix"}...]}
// shots.rs, appended at the end of SHOTS:
//   "station-fix"   Do(content), Do(|| { station::apply(b"fix s=food1 full>60 water>60", true); })   expect: toast_is(Toast::Station)
//   "station-wait"  the same tap twice                                                          expect: toast_is(Toast::Station) and the stored toast is TooSoon
```
```typst
// social.typ
#let station-presets() = ...   // table: place | raw(fix), from classic.at("stations").at("presets")
#let station-aliases() = ...   // table: raw(phrase) | raw(fix), from classic.at("stations").at("aliases")
```
- **Chapter 11, `== Stations: the magic places`**, rewritten. It covers:
  - A magic place has a reader. Hold the badge's back to it. The badge checks the signature, and the pet gets its fix. A phone without the key cannot do it.
  - The places: `#station-presets()`. The badge reads the message, not the place's name: a station can say what it fixes.
  - The old readers' phrases still work: `#station-aliases()`.
  - What a fix can do. Needs move only as far as the safe side of each danger line: food and water `#v("DMG_NEED_LOW_PCT")` to `#v("DMG_NEED_HIGH_PCT")`, energy from `#v("DMG_ENERGY_PCT")`, dirt up to `#v("DMG_DIRT_PCT")`, booze up to `#v("BOOZE_DANGER_PCT")`, strain up to `#v("STRAIN_INJ_PCT")`, weight `#v("DMG_WEIGHT_LOW_PCT")` to `#v("DMG_WEIGHT_HIGH_PCT")`. Fitness never goes past `#v("STATION_FIT_CAP_PCT")`, because real fitness is trained. Health goes up to the pet's own ceiling and never under `#v("HEALTH_DANGER_PCT")`. A need already past a line is only ever helped. Every fix adds `#v("STATION_MOOD_PCT")` mood.
  - Conditions a station can clear. The clinic is rare and can clear dependence and diabetes: "a reset, not a cure for a lifestyle".
  - The nap tent's energy fix means the next sleep cannot end groggy.
  - The rest: each place rests for `#v("STATION_CD_MIN")`, or longer when it asks, up to `#v("STATION_CD_MAX_MIN")`. The badge remembers `#v("STATION_COOLDOWN_SLOTS")` places, so two different places work back to back. A reboot clears the rests [Q4].
  - Who can use one: an egg counts, a frozen pet counts and stays frozen, a sleeping pet is not woken. A demo pet says #w("thanks!") and its own care waits `#v("DEMO_PAUSE_MIN")`. In hard mode a fix is free.
  - The toasts, as a table with `w()` on each: #w("Magic fix!") with #w("at <station>"), #w("thanks!"), #w("Covered by") / #w("magic, not insurance"), #w("Too soon!") / #w("again in <time>"), #w("Unknown magic") / #w("badge can't read it"), #w("pet is gone").
  - Pictures: `screen("station-fix")`, `screen("station-wait")`. Keep `#station-row()`.
- **Chapter 5:**
  - Delete the rows `Food bonus!`, `Heal bonus!` and `Sleep bonus!`, and add one row `[#w("Magic fix!")], [A station fixed the pet. See @ch-friends.]`.
  - The `wait 1:30` paragraph becomes one sentence: a station toast has two lines like a cause toast, see @ch-friends.
  - The never-raised sentence keeps only #w("+inspired").

- [ ] **Step 1: write the failing test.** In `tests/guide_shots.rs`, `every_shot_reaches_its_state` already covers new shots through `guide shots`. Add:
```rust
#[test] fn the_station_tables_come_from_code() {
    // `guide data classic`: stations.presets has 8 entries equal to scripts/station_presets.txt;
    // stations.aliases has fix::ALIASES.len() entries; values has STATION_MOOD_PCT, STATION_CD_MIN, STATION_COOLDOWN_SLOTS;
    // no key STATION_COOLDOWN, STATION_FOOD_FLOOR_PCT or STATION_BOOZE_CAP_PCT
}
```
- [ ] **Step 2: run and see it fail.** `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --test guide_shots`.
- [ ] **Step 3: implement** data, shots, the two typst functions and the two chapters.
- [ ] **Step 4: run and see it pass.** The same command: PASS. Then `CARGO_BUILD_JOBS=2 taskset -c 2,3 make guide`: green (marks, digits, pictures, pages). Look at the chapter 11 pages of the PDF.
- [ ] **Step 5: commit** `docs(guide): magic places in ch11, station toasts in ch5`.
- [ ] **Step 6: V.**

### Task 8: BadgeCtl, the field reader [Q2] (gated)

Run only when the owner has committed the local BadgeCtl changes and says go. Otherwise mark it skipped in the report. Work in `../../android_nfc/BadgeCtl` (its own repo, its current branch); never `git stash` there either.

Flash: 0.

**Files:**
- Create: `app/src/main/res/raw/station_presets.txt` (a copy of this repo's `scripts/station_presets.txt`).
- Modify: `app/src/main/java/com/example/badgectl/MainActivity.kt`, `app/src/main/res/layout/activity_main.xml`, `README.md`.

**Interfaces:**
- Consumes: the presets file, the aliases of Task 1, the status words of Task 4.
- Produces:
  - One button per preset, labelled with its place, in file order, built at start-up from `R.raw.station_presets` into `buttonsContainer`. They replace the four fixed buttons; the Inspire button goes.
  - `fallbackFor(plaintext)`: food1 → `more food`, pharm1 → `more drugs`, nap1 → `sleep like a bear`, else null. Hard-coded, matching `station.py fallback_for`.
  - `onTagDiscovered`: after `6A 82` with a fallback, run a fresh `fetchChallenge` plus a signed APDU with the fallback in the same IsoDep session, and report "sent the old phrase (old badge firmware)". Never fall back on `69 85`.
  - `handleResponse`: `0x6985 -> "badge refused: resting, pet gone or no pet (see the badge)"`; `0x6A82 -> "badge doesn't understand this message"`.

- [ ] **Step 1:** implement as above. There is no unit-test harness in BadgeCtl; the check is the build and the owner's tap test (Task 10).
- [ ] **Step 2:** `./gradlew assembleDebug` if an Android SDK is present. Otherwise report "not built: no SDK", and the owner builds it.
- [ ] **Step 3: commit in the BadgeCtl repo** `feat: magic-place presets with a fallback for old badge firmware` (footer as usual). Do not push.

### Task 9: regression and integration on CI

One push, the whole suite. Nothing changes in this task.

**Files:** none (report only).

**Interfaces:**
- Consumes: Tasks 1 to 7 (and 8 if run).
- Produces the regression report (in the task report, not a file):
  - every CI job's result, with failing test names and their assert messages;
  - the five station simulation `Outcome`s of both drivers (Task 5 `eprintln!`);
  - the SSD1680 release image size and its growth over `main`.

- [ ] **Step 1:** run locally, one command at a time: `CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo test -j 2 --features simulator --test station_v3`, `... --lib -- game::station:: game::tests::`, `... --test engine_v3 station_aliases::`, and `python3 -m unittest discover -s scripts -p 'test_station.py'`. Run the three firmware `cargo check` commands. Note failures; fix nothing yet.
- [ ] **Step 2:** `SSH_AUTH_SOCK=/run/user/1000/gcr/ssh git push github ranzbak/bornpets-v3-nfc`.
- [ ] **Step 3:** `gh run list -R ranzbak/bornhack-firmware-2026-ce -b ranzbak/bornpets-v3-nfc -L 1 --json databaseId,headSha`. The `headSha` must equal `git rev-parse HEAD`.
- [ ] **Step 4:** `gh run watch <id> -R ranzbak/bornhack-firmware-2026-ce --exit-status`, then `gh run view <id> -R ranzbak/bornhack-firmware-2026-ce --json conclusion,jobs`; for each failed job `--log-failed`.
- [ ] **Step 5:** check:
  - both firmware builds pass and the flash gate holds (≤ 960 KiB);
  - `station_v3` with `--include-ignored` is green;
  - the guide job is green, including `test_station.py`;
  - the growth is within the estimate (3 to 4.5 KiB).

  Measure the growth with `python3 scripts/flash_size.py` on this branch's release image against `main`'s, both built on CI or with the same command.
- [ ] **Step 6:** report. If everything is green, the coordinator may DFU-flash `fw-1680-release` to the owner's SSD1680 badge (owner allowance after review and green CI) for Task 10.

### Task 10: hardware check on a real badge (owner)

The owner taps; the agent prepares the badge and writes down what the owner reports. Needs: the owner's SSD1680 badge on this branch's `fw-1680-release`, with a hatched pet in normal (money) mode, plus the current BadgeCtl APK on a phone (old phrases). For checks 6 to 12 the owner also needs the Task 8 APK, or a USB PC/SC reader (for example ACR122U) running `python3 scripts/station.py send --key <signing_key.bin> --preset <place>`. Optional: a second badge still on old firmware.

| # | Tap | Badge should show | Reader should say |
|---|---|---|---|
| 1 | Badge on the Watch screen, old BadgeCtl **Feed** (`more food`) | jumps to the pet screen; `Magic fix!` / `at food`; stats: fullness and water at least 60 % | Sent (`90 00`) |
| 2 | **Feed** again within 5 minutes | `Too soon!` / `again in 4m` (or less), pet unchanged | rejected, `6985` |
| 3 | Old **Heal** (`more drugs`) | `Magic fix!` / `at drugs`; health at the pet's ceiling | Sent |
| 4 | Old **Sleep** (`sleep like a bear`) | `Magic fix!` / `at rest`; energy full; the pet stays awake | Sent |
| 5 | Old **Inspire** (`more inspiration`) [Q3] | `Magic fix!` / `at inspire` | Sent |
| 6 | New reader **Spa**, on a pet with floor items (dirt over 40 %) | `Magic fix!` / `at spa1`; the floor items are gone after the refresh | applied |
| 7 | New reader **Gym**, twice | first `Magic fix!` / `at gym1`; second `Too soon!` / `again in 3h59` | applied, then refused |
| 8 | New reader text `fix s=bad full>101` | `Unknown magic` / `badge can't read it` | not understood (`6A 82`), no fallback |
| 9 | Open the menu, then tap **Water point** | the menu closes; `Magic fix!` / `at water1` | applied |
| 10 | Settings: demo mode on, pet awake, tap **Food stall** | `Magic fix!` / `thanks!`; the pet does not care for itself for 30 minutes | applied |
| 11 | A hard-mode pet, tap **Pharmacy** | `Covered by` / `magic, not insurance`; HAX balance unchanged | applied |
| 12 | [Q1] Second badge on old firmware, new reader **Food stall** | its old toast `Food bonus!` | not understood, then "sent the old phrase" |
| 13 | A Gone pet (or let one leave), old **Feed** | `pet is gone` on the Gone screen | rejected, `6985` |

- [ ] **Step 1:** the coordinator flashes and sets up the pet, then hands the table to the owner.
- [ ] **Step 2:** record each row as pass or fail, with a photo or the owner's words for any fail. Every fail becomes an item for Task 11.

### Task 11: bug-fix round

Fix what Tasks 9 and 10 found, one item at a time.

**Files:** as each fix needs.

**Interfaces:**
- Consumes: the Task 9 report and the Task 10 table.
- Produces: fixes and a final green CI run.

For each failure:
- [ ] **Step 1:** reproduce it with a focused test (an existing one, or a new short one in the matching module). See it fail.
- [ ] **Step 2:** fix the cause (superpowers:systematic-debugging: root cause first).
- [ ] **Step 3:** run that focused test and the module it sits in. See them pass.
- [ ] **Step 4:** commit `fix(game): <what>` (one commit per item).

Fix rules:
- A Task 5 simulation bound missed: report the numbers to the owner; do not move the bound, and do not retune a station without the owner.
- Flash over 960 KiB: remove the Lights Out mini-game in its own commit (owner allowance), with its menu row, cooldown, docs, guide text and tests.
- A toast that does not fit on the badge: shorten the text within the toast geometry (labels may change), and update the guide's marks in the same commit.
- A wake miss: fix the engine's boundary, never add a periodic wake.

After the last fix:
- [ ] **Step 5:** push and watch CI as in Task 9, Steps 2 to 4. Repeat until green. Report the final numbers and the image size.

## On-screen words

| Where | Text |
|---|---|
| Station toast, line 1 (bold red, ≤ 13 columns) | `Magic fix!`, `Covered by`, `Too soon!`, `Unknown magic` |
| Station toast, line 2 (red, ≤ 21 columns) | `at <station>`, `thanks!`, `magic, not insurance`, `again in <time>`, `badge can't read it` |
| Reason toast (existing) | `pet is gone` |
| Removed | `Food bonus!`, `Heal bonus!`, `Sleep bonus!`, `Inspire bonus!`, `wait m:ss` |

## Self-review

**Spec coverage:**

| Spec item | Where |
|---|---|
| 19.1 today's three phrases and their limits | replaced: T1 (parser, 96 bytes), T4 (dispatcher) |
| 19.2 grammar, text not binary, lowercase, single spaces, 96 bytes | T1 (`normalize`, `parse`, vectors) |
| 19.2 operators `= > < + -` | T1 (`Op`), T2 (`apply`, `the_clamp_never_works_against_the_item`) |
| 19.2 clamped into the safe range per need; `health>100` to the ceiling | T2 (`safe_range` from the damage lines; `the_edges_are_the_damage_lines`) |
| 19.2 value over 100, missing id, malformed item reject the whole frame (`6A 82`) | T1 (`Over100`, `BadId`, `BadItem`), T4 (`Unknown`, SW) |
| 19.2 unknown name or condition skipped; nothing understood is rejected | T1 (vectors, `NothingKnown`) |
| 19.2 clinic clears dependence and diabetes; `fit>50` never lowers; station fit capped at 60 % | T2 (`conditions_clear_...`, `fit_is_capped_but_never_lowered`, `STATION_FIT_CAP_PCT`) |
| 19.2 several needs at one engine event | T2 (`several_needs_at_one_event`), T4 (one `with_game_now`) |
| 19.2 cooldown per station id, last 8 ids, RAM, 5 min default, `cd=` up to 1440 and never under 5 | T4 (`Cooldowns`, `cooldown_ticks`, eviction tests); T2 tunables; [Q4] |
| 19.2 +5 mood per accepted fix | T2 (`STATION_MOOD_PCT`, `every_fix_lifts_mood_by_the_bump`) |
| 19.3 old phrases as aliases with the given expansions; no weight, no stuffing, no sleeping, no chronic clears | T1 (`ALIASES`), T2 (migrated `station_aliases` tests); [Q3] adds a fourth |
| 19.4 delta engine: catch up, apply, recompute the wake | T4 (`with_game_now`), T2 (`a_fix_keeps_the_wake_exact`), T5 (oracle against delta) |
| 19.4 demo: player help, 30 min pause, thanks | T2 (`station_fix` calls `player_acted`), T3/T4 (`Thanks` toast; planner call 10) |
| 19.4 hibernation applied, time frozen; asleep applied without waking | T2 (`asleep_and_hibernating_pets_stay_so`) |
| 19.4 care score, battle snapshot | nothing to build: mood follows, the next beacon reads the new state (plan 5's snapshot is taken at challenge time) |
| 19.4 hard mode free, toast `Covered by magic. Not by your insurance.` | T2 (`hard_mode_pays_nothing`), T3 (`Covered`, shortened to fit: planner call 8) |
| 19.4 station energy fix suppresses the next groggy check | T2 (`an_energy_fix_forgives_the_next_oversleep`), carried from plan 1 (`no_groggy`) |
| 19.5 reader side: the string only; the eight presets | T1 (`station_presets.txt`), T6 (`station.py`, docs), T8 [Q2] |
| 19.6 simulation of the three alias stations daily | T5 (ignored runs with both drivers) |
| 19.7 `station.rs` replaced; `GameState::station_fix`; `nfct.rs` and signed channel untouched apart from the call | T2, T4. `nfct.rs` changes only its two call sites and the status word [Q1] |
| 19.7 `NFC_README.md` and the reader tool get the new strings | T6, T8 |
| 19.7 tests: presets at both extremes never in danger; old phrases accepted; unknown skipped; malformed rejected | T2 (`no_fix_raises_health_damage`, edges), T1 (vectors, aliases) |
| Roadmap: tunables with Classic / Casual and a CFG key | T2 (four fields, template, key list) |
| Roadmap: updates guide ch 11 | T7 |
| Owner: a refusal says why | T3, T4 (`every_refusal_raises_a_toast`) |
| Owner: old badges and tags keep working | T1 aliases; [Q1] fallback in T6 and T8; T10 rows 1 to 5 and 12 |
| Owner: hardware check by the owner | T10 |

**Planner calls:**
1. **The parser lives in the engine** (`engine/fix.rs`, pure and host-tested). `station.rs` keeps the RAM table and the glue. The spec's `GameState::station_fix(&[Item])` takes a `&Fix`, so the conditions and the id travel with the items.
2. **Whitespace:** any run of ASCII whitespace counts as one space (organisers type strings by hand). The 96-byte limit is checked after collapsing.
3. **Duplicates:** a second `s=` or `cd=` rejects the frame. Several `clear=` tokens add up. The same need twice is applied in order.
4. **An unknown name** is skipped only when its token is well formed (`name op rest`). `cd=` alone does not count as "understood".
5. **The clamp limits the move, never reverses it.** A need already past an edge is left alone or helped. The fit cap therefore never lowers a fitter pet, as 19.2 asks for `fit>50`.
6. **Safe ranges are the health-damage lines** (`DMG_*`, `BOOZE_DANGER_PCT`, `STRAIN_INJ_PCT`, `HEALTH_DANGER_PCT`). In Classic they equal the spec's numbers, and in Casual they follow Casual's lines. One source of truth, no new tunables for them. The health floor (30 %) is the danger line, because the spec gives no lower bound.
7. **Four new tunables.** Casual equals Classic: stations are festival infrastructure, and Casual already moves the damage lines.
8. **Toast texts** (owner allows renames): the spec's hard-mode line is too long for the toast. It becomes `Covered by` / `magic, not insurance`. The old four single-line bonus toasts are replaced by one two-line form that names the station.
9. **The unsigned opt-in path** (`nfc-plaintext-station`, off in every shipped build) skips `clear=dependence,diabetes`: the clinic needs a signed reader. Any station message on that path is transient, refused or not, which fixes today's profile bug.
10. **Demo thanks lives in the station toast**, not in a `Cause::Thanks`. A queued cause would replace the station toast on the next `cycle`. Thanks shows only for an awake, not-hibernating demo pet.
11. **Eggs take a fix** (spec 19.1 and 19.4, guide ch 11 "an egg counts"). The egg's needs carry into the hatched pet. Toasts are now drawn on the Hatching screen.
12. **No game at all:** the reply is `69 85`, and the start screen is brought forward without a toast. The start screen already says what to do. A Gone pet shows `pet is gone` on the Gone screen.
13. **Cooldown slots store the id bytes, not a hash** (no collisions; 8 × 13 bytes of RAM). Eviction prefers an empty or free slot, then the slot that frees soonest. Only an applied fix stamps.
14. **The tap closes an open menu and the Friends, Diary and Traits views.** It shares `bring_game_forward` with plan 5's challenge. A pre-battle screen, a mini-game or text entry stay open; the toast then follows the existing rule for toasts behind a full-screen view.
15. **`StationInspire` and `Inspire bonus!` go.** They were never raised (R15).
16. **No simulator key for stations.** The guide shots and `station::apply` tests cover the simulator path. A key binding would be UI nobody asked for.
17. **A cause from the catch-up can replace the success toast** (for example a pet that self-wakes groggy in the same update). It is rare, the reader still reports success, and refusals never run a catch-up.

**Type consistency:** checked `Fix`, `Fix::new`, `Fix::id`, `Fix::understood`, `Fix::canonical`, `Level`, `Target`, `Op`, `cond::*`, `Reject`/`code`, `FIX_MAX_LEN`, `STATION_ID_MAX`, `MAX_LEVELS`, `ALIASES`, `normalize`, `expand_alias`, `parse`, `parse_text`, `safe_range`, `fix::apply`, `GameState::station_fix` (returns the thanks flag), `StationToast`, `station_lines`, `show_station`, `bring_game_forward`, `Toast::Station`, `last_station_toast_for_test`, `Cooldowns`/`remaining`/`stamp`, `COOLDOWN_SLOTS`, `cooldown_ticks`, `Outcome`/`sw`/`is_station`, `station::apply(text, signed)`, `game_phase`, `Act::Station`, `with_stations`, `stations_only`, and the four keys across the tasks that use them. The Python names mirror the Rust ones, and the vectors file is their shared contract.

**Review Focus:** items 1 to 5 are pinned in T2, T3/T4, T4, T4 and T2/T5.

**Proportion:** 7 build tasks, a gated reader task, CI, the owner's check and a fix round. Parsing (T1) and applying (T2) are split so a reviewer can reject the clamp rules without the grammar. The toasts (T3) are split from the dispatcher (T4) because T3 also touches plan 5's view.

**Flash:** estimated 3 to 4.5 KiB in total, measured in Task 9:
- T1 parser and aliases, about 1.5 KiB;
- T2 apply and tunables, about +0.7 KiB net;
- T3 toasts, about 0.5 KiB;
- T4 dispatcher, about +0.4 KiB net;
- T5 to T8, 0 (host, scripts, guide, the other repo).

Lights Out stays unless Task 9 shows the gate is hit.

**Refresh and battery:** a tap raises one toast. That is one partial refresh to show it and one to clear it after 2 s, as for any toast. A fix changes the engine state once, and the next wake is computed from it: no new periodic wake and no full refresh.
