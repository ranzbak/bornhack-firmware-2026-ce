# BornPets v2 roadmap

Spec: `BORNPETS_V2_DESIGN.md` (repo root). Current rules and results: sections 15 to 19 (18 and 19 win where they differ from earlier sections; 17 and 18.8 are the owner's decisions).
Code base: worktree `../BornHack-Cyberegg-fable6`, branch `ranzbak/fable-06-layering`, HEAD `710371f`. All paths and line numbers in the plans refer to that tree. v2 lands after that cleanup.
Each plan ships working firmware and a working simulator on its own, with its tests green.

## Order

| # | Plan | Ships | Depends on | Spec sections | Size |
|---|---|---|---|---|---|
| 1 | **Engine core and save v3** | the v2 needs, doses, interactions, conditions, sleep, delta solver, 2 h wake cap, save v3 with v2 upgrade, simulator with step oracle; just enough menu items to play it (Wash, Relax, Wake up, Water refusal toasts) | none | 1 to 4, 13, 15.1 to 15.6, 15.8, 17 rows 1 to 10 and 28 to 31, 18.1 to 18.3 | L |
| 2 | **Reading the pet (UI)** | two-page stats view with band ticks, status line, cause toasts, menu hints, diary, Health status entry removed, 5-minute idle frame | 1 | 5, 17 rows 29 and 32, 18.6 | M |
| 3 | **Demo mode** | self-caring pet, clock-following sleep, fitness floor, settings toggle, calm display | 1 (engine events), 2 (status line) | 8, 13.3, 17 rows 11 to 15, 18.3, 18.8 | M |
| 4 | **NFC stations** | `fix s=...` plaintext, old phrases as aliases, per-station cooldown, clinic preset | 1 | 19 | S |
| 5 | **Battles** | new battle formula and floors, snapshot, challenge / accept / decline, pre-battle screen with win chance, 100 / 20 HAX pay, beacon with six numbers | 1; 2 for the mirrored bar widget | 12, 15.7, 17 rows 16 to 23a, 18.4, 18.5, 18.8 | L |
| 6 | **Hard mode and money** | v2 prices and work, debt, deductible and copay, work insurance, insulin creep (cap 60), surprise bills, authorization popup, eviction | 1, 2 (bill and popup screens) | 15.9, 16, 17 rows 24 to 27, 18.5 | M |
| 6b | Crowdfunding and cross-border pharmacy | two SHDW message types | 5, 6 | 16.3 rows 10 and 11 | M |
| 7 | **Backgrounds, floor items, Jellyfish removal** | layered backgrounds, day and night from the clock, floor items tied to dirt | 1 (dirt), 2 (screen layout) | 14 (not written by this planner; read it before planning) | M |
| 8 | Long-term layer | care score display, growth forms as titles, traits as modifiers, Unicorn Realm record fields | 1, 2 | 4, 17 row 11 | S to M |

Plans 3, 4, 5 and 7 are independent of each other once 1 and 2 are in. 6 needs the work action from 1 and screens from 2. Suggested sequence for a festival deadline: 1, 2, 4, 3, 6, 5, 7, 8, 6b.

## Save format across plans

Plan 1 defines save v3 once, with 32 reserved zero bytes, so plans 3 (demo timers), 5 (per-pair cooldown lives in the friends blob, not here) and 6 (hard-mode counters, 17 bytes) fill reserved bytes without another version bump. Plan 1 owns the v2 to v3 upgrade.

## Are the old "phase 1" tuning fixes still worth shipping before v2?

Mostly no. They patch the v1 engine that plan 1 replaces: the softer sick rates, the Only-pets change and the mini-game reward conditions would be thrown away.

Worth doing only if a firmware release goes out before plan 1 lands, because each is a few lines and fixes a real exploit or a visible bug:

1. Aspirine cooldown no longer suppresses the diabetes and alcoholism penalties (`src/game/engine/mod.rs:983` and `:1062`).
2. The wrong toast on Battle (`src/game/modal.rs:425`).
3. The doc corrections of the audit's section 5 (`GAME.md`, `GAMES.md`, `USER_GAMES.md`), which stay true until v2 replaces those docs.

Skip the rest.

## Not planned yet

- Catch-up of at most 12 h on boot (17 row 30) needs the watch clock at game init; it belongs to plan 1's `lifecycle::try_load` but depends on how the RTC survives power-off, which the spec does not settle. It is a task in plan 1 guarded on "clock known".
- Mesh abuse limits for 6b.
