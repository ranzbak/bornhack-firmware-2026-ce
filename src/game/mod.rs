//! BornPets — game screen renderer.
//!
//! Layout (152×152 EPD, black & white + red):
//!
//! ```text
//! ┌───────────────────────────────────────┐  y =  0
//! │  [Stats] [Hibernate]  (empty)(empty)  │  y = 0–34   top icon row
//! ├───────────────────────────────────────┤  y = 34
//! │                                       │
//! │            [pet / egg]                │  y = 35–110  pet area
//! │                                       │
//! ├───────────────────────────────────────┤  y = 111
//! │  [Feed]  [Heal]  [Play]  [Rest]       │  y = 111–152 bottom icon row
//! └───────────────────────────────────────┘  y = 152
//! ```

pub mod battle;
pub mod battle_view;
pub mod blackhole;
pub mod bornjeweled;
pub mod debug_cheats;
pub mod diary;
pub mod diary_view;
pub mod engine;
pub mod friends;
pub mod friends_view;
pub mod health_view;
pub mod input;
pub mod lifecycle;
pub mod lightsout;
pub mod minigame;
pub mod modal;
pub mod nav;
pub mod nim;
pub mod bornpets_cfg;
pub mod pacing;
pub mod pet_registry;
pub mod pets_cfg;
pub mod pet_select;
pub mod realm_view;
pub mod settings;
pub mod sprite_loader;
pub mod stat_bar;
pub mod station;
pub mod tictactoe;
#[cfg(test)]
mod test_canvas;
pub mod timed_flag;
pub mod traits_view;
// ── Action feedback toast ────────────────────────────────────────────────────
use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, Ordering};

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
pub use nav::{GameNav, Row};

use crate::{BLACK, TriColor};
use timed_flag::TimedFlag;

/// Action feedback shown briefly after an action.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
#[repr(u8)]
pub enum Toast {
    None = 0,
    Feed = 1,
    Heal = 2,
    Sleep = 3,
    Relax = 4,
    Play = 5,
    Inspired = 6,
    Hibernate = 7,
    Wake = 8,
    StationFood = 9,
    StationDrugs = 10,
    StationInspire = 11,
    StationRest = 12,
    /// Station tap was rejected because the matching effect is still
    /// on cooldown.  The remaining seconds are read from
    /// `STATION_COOLDOWN_SECS` and formatted at draw time.
    StationCooldown = 13,
    /// Took Ozempic — weight + hunger relief, distinct from Exercise.
    /// Reuses discriminant 14 (freed when Triple Born was removed).
    Ozempic = 14,
    Exercise = 15,
    Medicate = 16,
    DebugCheat = 17,
    /// Drank something alcoholic (Beer/Wine/Whiskey) — raises `drunk`.
    Drink = 18,
    Rehab = 19,
    NewFriend = 20,
    FriendReunion = 21,
    /// Shown when we were the *target* of a friend's mesh Battle
    /// challenge and it resolved in our favor — see `game::battle`.
    BattleWon = 22,
    BattleLost = 23,
    /// Drank something non-alcoholic (Water/Cola) — never touches
    /// `drunk`, unlike `Drink`. Kept as its own variant so the toast
    /// after picking Water/Cola doesn't misleadingly say "+drunk".
    Refreshed = 24,
    /// Sent the pet to "Only pets" — see `game::engine::Action::OnlyPets`.
    OnlyPets = 25,
    /// Won (or drew) a mini-game — see `GameState::award_inspiration`.
    /// Neutral flavor toast; replaced the old `Inspired` ("+inspired")
    /// toast when the `drained`/inspired stat was removed.
    MinigameWin = 26,
    /// Why an action was refused or a row is disabled; the text comes from
    /// the stored `engine::doses::Reason` (see `show_reason`).
    Reason = 27,
    /// Started a wash — see `engine::Action::Wash`.
    Wash = 28,
    /// Debug menu "Engine info": two lines read from the engine at draw time.
    DebugInfo = 29,
    /// Why something went badly: two lines (what, why) from `Cause`, read
    /// from `CAUSE_CODE` (see `show_cause`).
    Cause = 30,
}

impl Toast {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Feed,
            2 => Self::Heal,
            3 => Self::Sleep,
            4 => Self::Relax,
            5 => Self::Play,
            6 => Self::Inspired,
            7 => Self::Hibernate,
            8 => Self::Wake,
            9 => Self::StationFood,
            10 => Self::StationDrugs,
            11 => Self::StationInspire,
            12 => Self::StationRest,
            13 => Self::StationCooldown,
            14 => Self::Ozempic,
            15 => Self::Exercise,
            16 => Self::Medicate,
            17 => Self::DebugCheat,
            18 => Self::Drink,
            19 => Self::Rehab,
            20 => Self::NewFriend,
            21 => Self::FriendReunion,
            22 => Self::BattleWon,
            23 => Self::BattleLost,
            24 => Self::Refreshed,
            25 => Self::OnlyPets,
            26 => Self::MinigameWin,
            27 => Self::Reason,
            28 => Self::Wash,
            29 => Self::DebugInfo,
            30 => Self::Cause,
            _ => Self::None,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Toast::None => "",
            Toast::Feed => "+full",
            Toast::Heal => "healing",
            Toast::Sleep => "sleeping",
            Toast::Relax => "calmer +energy",
            Toast::Wash => "cleaner",
            Toast::Play => "+fun",
            Toast::Inspired => "+inspired",
            Toast::Hibernate => "hibernating",
            Toast::Wake => "waking up",
            Toast::StationFood => "Food bonus!",
            Toast::StationDrugs => "Heal bonus!",
            Toast::StationInspire => "Inspire bonus!",
            Toast::StationRest => "Sleep bonus!",
            // Dynamic — handled in the renderer.
            Toast::StationCooldown => "",
            Toast::Ozempic => "-weight",
            Toast::Exercise => "-weight",
            Toast::Medicate => "+medicated",
            Toast::DebugCheat => "cheat applied",
            Toast::Drink => "+drunk",
            Toast::Rehab => "rehab started",
            Toast::NewFriend => "new friend!",
            Toast::FriendReunion => "+happy",
            Toast::BattleWon => "won a battle!",
            Toast::BattleLost => "lost a battle",
            Toast::Refreshed => "refreshed!",
            Toast::OnlyPets => "Only pets!",
            Toast::MinigameWin => "nice game!",
            // Dynamic — handled in the renderer.
            Toast::Reason => "",
            // Dynamic — handled in the renderer.
            Toast::DebugInfo => "",
            // Two lines, drawn by `draw_cause_toast`.
            Toast::Cause => "",
        }
    }
}

/// Toast message index.
static TOAST_MSG: AtomicU8 = AtomicU8::new(0);
/// Whether a toast is currently visible.  Cleared on the first
/// render that occurs at or after `TOAST_MIN_VISIBLE_MS` since it was shown.
static TOAST: TimedFlag = TimedFlag::new();

/// The reason shown by [`Toast::Reason`]: `Reason::pack` kind and seconds.
static REASON_KIND: AtomicU8 = AtomicU8::new(0);
static REASON_SECS: AtomicU32 = AtomicU32::new(0);

/// The cause shown by [`Toast::Cause`]: `Cause::pack`.
static CAUSE_CODE: AtomicU8 = AtomicU8::new(0);

/// Remaining cooldown in seconds, read by the renderer when the
/// active toast is [`Toast::StationCooldown`].  Set by
/// [`show_station_cooldown`].
static STATION_COOLDOWN_SECS: AtomicU16 = AtomicU16::new(0);

/// Minimum wall-clock visibility for a toast.  After this elapses the
/// toast disappears on the next display refresh.  E-paper refreshes
/// (fast LUT or full) take roughly 0.5–3 s each, so the actual
/// on-screen time is `max(TOAST_MIN_VISIBLE_MS, time-to-next-refresh)`.
const TOAST_MIN_VISIBLE_MS: u32 = 2000;

/// Whether the full-screen "now diabetic" alert is currently taking
/// over the display.  Set by [`show_diabetes_alert`], cleared once
/// `DIABETES_ALERT_MIN_VISIBLE_MS` has elapsed since it started — same
/// start-timestamp-plus-minimum-duration pattern as the toast above,
/// just occupying the whole screen instead of one line.
static DIABETES_ALERT: TimedFlag = TimedFlag::new();
/// Minimum wall-clock visibility for the diabetes alert screen.
const DIABETES_ALERT_MIN_VISIBLE_MS: u32 = 3000;

/// Show the full-screen "now diabetic" alert, taking over the whole
/// display (icons, pet, everything) for at least
/// `DIABETES_ALERT_MIN_VISIBLE_MS`. Called once, the instant diabetes
/// triggers — see `lifecycle::check_diabetes_onset`.
pub fn show_diabetes_alert() {
    DIABETES_ALERT.start();
    #[cfg(feature = "embassy-core")]
    crate::TOAST_SIGNAL.signal(());
}

/// Full-screen "now alcoholic" alert — the alcoholism twin of the
/// diabetes onset takeover above, reusing `DIABETES_ALERT_MIN_VISIBLE_MS`.
static ALCOHOLISM_ALERT: TimedFlag = TimedFlag::new();

/// Show the full-screen "now alcoholic" alert, fired once the instant
/// alcoholism triggers — see `lifecycle::check_alcoholism_onset`.
pub fn show_alcoholism_alert() {
    ALCOHOLISM_ALERT.start();
    #[cfg(feature = "embassy-core")]
    crate::TOAST_SIGNAL.signal(());
}

// ── Battle animation takeover ────────────────────────────────────────────────
//
// A full-screen two-stage battle result (see the battle-animation spec). Unlike
// the alerts above it takes over the *whole* display regardless of the active
// screen, so the main display loop checks `battle_anim_active()` before drawing
// its normal screen. Two stages of `BATTLE_STAGE_MS` each: standing, then the
// won/lost poses.

/// Duration of one battle-animation stage.
const BATTLE_STAGE_MS: u32 = 5_000;
/// Total animation length — both stages back to back.
const BATTLE_TOTAL_MS: u32 = 2 * BATTLE_STAGE_MS;

static BATTLE_ANIM: TimedFlag = TimedFlag::new();
/// Packed context: byte0 = own pet kind, byte1 = opponent kind, bit16 =
/// viewer-won. Set once by [`show_battle_anim`], read by the renderer.
static BATTLE_ANIM_CTX: AtomicU32 = AtomicU32::new(0);
/// Opponent device id, packed little-endian into a u16. Kept out of
/// `BATTLE_ANIM_CTX` (which is full) — the renderer uses it to look the
/// opponent's pet name up in the friends list for the on-screen labels.
static BATTLE_ANIM_OPP_ID: AtomicU16 = AtomicU16::new(0);

/// Which stage of the battle animation to draw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BattleStage {
    /// Both pets standing.
    Standing,
    /// Winner/loser poses + banner.
    Result,
    /// Animation finished — the takeover should end.
    Done,
}

/// Pure mapping from elapsed milliseconds to the current [`BattleStage`].
pub fn battle_stage_from_elapsed(ms: u32) -> BattleStage {
    if ms < BATTLE_STAGE_MS {
        BattleStage::Standing
    } else if ms < BATTLE_TOTAL_MS {
        BattleStage::Result
    } else {
        BattleStage::Done
    }
}

/// Start the full-screen battle animation. `own_kind` is drawn on the left,
/// `opp_kind` on the right (mirrored); `viewer_won` picks the stage-2 poses.
/// `own_hp`/`opp_hp` are the outcome HP percentages (0..=100) shown by the
/// stage-2 power bars — stage 1 always shows 100%.
///
/// `opp_id` is the opponent's device id, used only to label them on screen.
///
/// Layout of the packed context (`BATTLE_ANIM_CTX`, all in one u32):
/// bits 0-7 own_kind, 8-15 opp_kind, 16-22 own_hp, 23-29 opp_hp, 30 viewer_won.
pub fn show_battle_anim(
    own_kind: u8,
    opp_kind: u8,
    viewer_won: bool,
    own_hp: u8,
    opp_hp: u8,
    opp_id: [u8; 2],
) {
    let ctx = own_kind as u32
        | ((opp_kind as u32) << 8)
        | ((own_hp.min(100) as u32) << 16)
        | ((opp_hp.min(100) as u32) << 23)
        | ((viewer_won as u32) << 30);
    BATTLE_ANIM_CTX.store(ctx, Ordering::Relaxed);
    BATTLE_ANIM_OPP_ID.store(u16::from_le_bytes(opp_id), Ordering::Relaxed);
    BATTLE_ANIM.start();
    #[cfg(feature = "embassy-core")]
    crate::TOAST_SIGNAL.signal(());
}

/// True while the battle-animation takeover should own the display.
pub fn battle_anim_active() -> bool {
    BATTLE_ANIM.is_active()
}

/// Current stage from the elapsed time since [`show_battle_anim`].
pub fn battle_anim_stage() -> BattleStage {
    battle_stage_from_elapsed(BATTLE_ANIM.elapsed_ms())
}

/// Unpacked animation context: `(own_kind, opp_kind, viewer_won)`.
pub fn battle_anim_ctx() -> (u8, u8, bool) {
    let c = BATTLE_ANIM_CTX.load(Ordering::Relaxed);
    (c as u8, (c >> 8) as u8, (c >> 30) & 1 != 0)
}

/// Opponent device id, for resolving their name in the friends list.
pub fn battle_anim_opp_id() -> [u8; 2] {
    BATTLE_ANIM_OPP_ID.load(Ordering::Relaxed).to_le_bytes()
}

/// Outcome HP percentages `(own_hp, opp_hp)` for the stage-2 power bars.
pub fn battle_anim_hp() -> (u8, u8) {
    let c = BATTLE_ANIM_CTX.load(Ordering::Relaxed);
    (((c >> 16) & 0x7F) as u8, ((c >> 23) & 0x7F) as u8)
}

/// End the battle-animation takeover (called once the stage reaches `Done`).
pub fn clear_battle_anim() {
    BATTLE_ANIM.clear();
}

/// Show a feedback toast.  Becomes visible on the next display
/// refresh and stays visible until the first refresh at or after
/// `TOAST_MIN_VISIBLE_MS` elapsed.  Fires `TOAST_SIGNAL` so the
/// display loop wakes up immediately even if no other event would
/// have triggered a redraw (e.g. an NFC station bonus arriving while
/// the display task is parked).
pub fn show_toast(toast: Toast) {
    TOAST_MSG.store(toast as u8, Ordering::Relaxed);
    TOAST.start();
    #[cfg(feature = "embassy-core")]
    crate::TOAST_SIGNAL.signal(());
}

/// The cause toast: what went wrong (bold red, on the HAX row), then why
/// (red, one row lower, clear of the balance).
fn draw_cause_toast<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = crate::TriColor>,
{
    let Some(cause) = engine::causes::Cause::unpack(CAUSE_CODE.load(Ordering::Relaxed)) else {
        return Ok(());
    };
    let style = crate::ui::TOP_LEFT;
    crate::ui::draw_text(display, cause.what(), TOAST_POS, crate::ui::TEXT_BOLD_RED, style)?;
    let second = TOAST_POS + Point::new(0, TOAST_LINE_DY);
    crate::ui::draw_text(display, cause.why(), second, crate::ui::TEXT_RED, style)?;
    Ok(())
}

/// Full-screen one-shot condition alert: two bold black lines and a red hint,
/// cleared once `ALERT_MIN_VISIBLE_MS` has elapsed.
fn draw_alert<D>(
    display: &mut D,
    flag: &TimedFlag,
    lines: [&str; 2],
    hint: &str,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    
    
    use embedded_graphics::text::Text;

    let centered = crate::ui::CENTERED;
    let font = crate::ui::TEXT_BOLD_BLACK;
    let font_red = crate::ui::TEXT_BOLD_RED;
    crate::ui::clear_screen(display)?;
    Text::with_text_style(lines[0], Point::new(76, 56), font, centered).draw(display)?;
    Text::with_text_style(lines[1], Point::new(76, 74), font, centered).draw(display)?;
    Text::with_text_style(hint, Point::new(76, 100), font_red, centered).draw(display)?;
    flag.clear_if_elapsed(DIABETES_ALERT_MIN_VISIBLE_MS);
    Ok(())
}

/// True when the game screen is showing a red status message — an active
/// toast, or the "gone / new egg" prompt.  The display loop uses this to run
/// a genuine full (tri-color) refresh instead of the fast delta refresh: red
/// ink under-drives on the non-inverting delta LUT (LUT2), so red text only
/// seats properly on the full-waveform path.  `partial_idle` upstream keeps
/// this from re-flashing once the message is drawn.
pub fn status_wants_full_refresh() -> bool {
    TOAST.is_active()
        || DIABETES_ALERT.is_active()
        || ALCOHOLISM_ALERT.is_active()
        || lifecycle::display_anim() == engine::DisplayAnim::Gone
        || (!lifecycle::is_hibernating()
            && lifecycle::status_line().is_some_and(|l| l.danger))
        || battle_view::wants_full_refresh()
}

/// Tell the player why an action was refused or a row is disabled.
pub fn show_reason(why: engine::doses::Reason) {
    let (kind, secs) = why.pack();
    REASON_KIND.store(kind, Ordering::Relaxed);
    REASON_SECS.store(secs, Ordering::Relaxed);
    show_toast(Toast::Reason);
}

/// Tell the player what went wrong and why (silent: no buzz).
pub fn show_cause(c: engine::causes::Cause) {
    CAUSE_CODE.store(c.pack(), Ordering::Relaxed);
    show_toast(Toast::Cause);
}

/// The toast that is up now.
#[cfg(test)]
pub(crate) fn current_toast_for_test() -> Toast {
    if TOAST.is_active() {
        Toast::from_u8(TOAST_MSG.load(Ordering::Relaxed))
    } else {
        Toast::None
    }
}

/// The cause the last `show_cause` stored.
#[cfg(test)]
pub(crate) fn last_cause_for_test() -> Option<engine::causes::Cause> {
    engine::causes::Cause::unpack(CAUSE_CODE.load(Ordering::Relaxed))
}

/// The reason the last `show_reason` stored.
#[cfg(test)]
pub(crate) fn last_reason_for_test() -> engine::doses::Reason {
    engine::doses::Reason::unpack(
        REASON_KIND.load(Ordering::Relaxed),
        REASON_SECS.load(Ordering::Relaxed),
    )
}

/// Tick the live view was last drawn (or opened); its refresh deadline is
/// anchored here so minute ticks cannot slide it forward.
static LIVE_DRAWN_TICK: AtomicU32 = AtomicU32::new(0);

/// Record that a live view was just drawn.
pub(crate) fn note_live_view_drawn() {
    LIVE_DRAWN_TICK.store(lifecycle::now_tick(), Ordering::Relaxed);
}

/// The next refresh tick of a live view last drawn at `drawn`: on the grid
/// of idle frames anchored at the draw, so it is never in the past (a view
/// hidden under another screen cannot cause a poll) and at most one idle
/// frame ahead.
fn live_deadline(drawn: u32, now: u32) -> u32 {
    let frames = now.saturating_sub(drawn) / pacing::IDLE_FRAME_TICKS + 1;
    drawn.saturating_add(pacing::IDLE_FRAME_TICKS.saturating_mul(frames))
}

/// True while a view whose numbers drift by themselves is on screen. Such a
/// view is refreshed at least once per idle frame (display pacing, not
/// engine polling). A Gone pet shows no live view.
pub fn live_view_open() -> bool {
    (modal::live_modal_view_open() || diary_view::is_active()) && lifecycle::display_anim() != engine::DisplayAnim::Gone
}

/// Next tick the frame loop must wake for UI-only timed state, whatever the
/// engine says: a toast or alert is cleared by the first draw after its
/// minimum time, and the sleep animation floor ends at a fixed tick. Without
/// this the 2 h engine cap would leave them on screen for hours. The battle
/// result card is NOT here: it waits for a button press, which redraws, and
/// polling it would wake the badge every tick for as long as it is left up.
pub fn next_ui_deadline_tick() -> u32 {
    let now = lifecycle::now_tick();
    let overlay = TOAST.is_active()
        || DIABETES_ALERT.is_active()
        || ALCOHOLISM_ALERT.is_active();
    let mut next = lifecycle::sleep_anim_deadline_tick();
    if overlay {
        next = next.min(now.saturating_add(1));
    }
    if live_view_open() {
        let drawn = LIVE_DRAWN_TICK.load(Ordering::Relaxed);
        next = next.min(live_deadline(drawn, now));
    }
    next
}

/// Show the station-cooldown toast with the remaining time formatted
/// from `secs` (e.g. `"wait 4:50"`).
pub fn show_station_cooldown(secs: u16) {
    STATION_COOLDOWN_SECS.store(secs, Ordering::Relaxed);
    show_toast(Toast::StationCooldown);
}

// ── Layout constants
// ──────────────────────────────────────────────────────────

/// X centres of the four icon columns (evenly spaced across 152 px).
const ICON_CX: [i32; 4] = [19, 57, 95, 133];
/// Y centre of the top icon row.
const TOP_CY: i32 = 17;
/// Y centre of the bottom icon row.
const BOT_CY: i32 = 131;
/// Y of the separator below the top icon row.
const SEP_TOP: i32 = 34;
/// Y of the separator above the bottom icon row.
const SEP_BOT: i32 = 111;
/// Status line: x of the centre of every line (panel is 152 px wide).
const STATUS_CENTER_X: i32 = 76;
/// Status line: y of the bottom row of the last line, two px above the
/// separator.
const STATUS_BASE_Y: i32 = SEP_BOT - 2;
/// Status line: height of one text line (FONT_7X13_BOLD).
const STATUS_LINE_H: i32 = 13;
/// Status line: width of one column (FONT_7X13_BOLD).
const STATUS_CHAR_W: i32 = 7;
/// Status line: extra white backing width, split evenly left and right.
const STATUS_BACKING_PAD: i32 = 4;

/// First display row of the pet/sprite area.
/// The HAX balance: right-aligned, top baseline, ending at x = 150.
const HAX_POS: Point = Point::new(150, 33);
/// The toast's first line, top-left. It shares the HAX row, so the cause
/// toast keeps its `what` line short enough to end left of the balance.
const TOAST_POS: Point = Point::new(2, SEP_TOP + 2);
/// Distance from a toast's first line to its second.
const TOAST_LINE_DY: i32 = 13;
pub const PET_AREA_TOP: usize = SEP_TOP as usize + 1;

// ── Public entry point
// ────────────────────────────────────────────────────────

/// Draw the status line at the bottom of the pet area: bottom-centred, bold
/// black (bold red in danger), each line on a white backing of its own width
/// plus [`STATUS_BACKING_PAD`] so it reads over the sprite. A second line sits
/// above the first.
pub fn draw_status_line<D>(
    display: &mut D,
    line: &engine::status::StatusLine,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let text = line.text();
    let style = if line.danger {
        crate::ui::TEXT_BOLD_RED
    } else {
        crate::ui::TEXT_BOLD_BLACK
    };
    let count = text.split('\n').count() as i32;
    for (i, row) in text.split('\n').enumerate() {
        let base = STATUS_BASE_Y - (count - 1 - i as i32) * STATUS_LINE_H;
        let w = row.chars().count() as i32 * STATUS_CHAR_W + STATUS_BACKING_PAD;
        Rectangle::new(
            Point::new(STATUS_CENTER_X - w / 2, base - (STATUS_LINE_H - 1)),
            Size::new(w as u32, STATUS_LINE_H as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(crate::WHITE))
        .draw(display)?;
        crate::ui::draw_text(
            display,
            row,
            Point::new(STATUS_CENTER_X, base),
            style,
            crate::ui::CENTERED_BOTTOM,
        )?;
    }
    Ok(())
}

/// Render the BornPets game screen.
///
/// Handles four states:
/// - **Not started**: "Press Fire to start" — no icons.
/// - **Hatching**: egg animation + countdown — no icons.
/// - **Gone**: farewell + "Press Execute for new egg" — no icons.
/// - **Active**: icons + pet animation + modal overlay.
pub fn draw_screen_game<D>(display: &mut D, nav: GameNav) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    
    
    use embedded_graphics::text::{Text};
    use engine::to_display::DisplayAnim;

    // `nav` only drives the simulator's in-line icon blit (firmware
    // does that work in the async pre-pass `render()` instead).  Suppress
    // the unused-variable warning for non-simulator builds.
    #[cfg(not(feature = "simulator"))]
    let _ = &nav;

    let centered = crate::ui::CENTERED;
    let font = crate::ui::TEXT_BOLD_BLACK;
    // Red style for pet status prompts (the "gone / new egg" screen) so they
    // stand out from the black countdown timer / labels.
    let font_red = crate::ui::TEXT_BOLD_RED;

    // ── Full-screen takeover screens ───────────────────────────────────
    // Diabetes onset alert takes priority over everything else, including
    // mini-games and modals — a rare, one-shot event worth interrupting
    // whatever's on screen for.
    if DIABETES_ALERT.is_active() {
        return draw_alert(
            display,
            &DIABETES_ALERT,
            ["TYPE 2", "DIABETES"],
            "Give medication soon",
        );
    }

    // Alcoholism onset alert — same one-shot full-screen takeover as
    // diabetes, the twin permanent condition on the drink arc.
    if ALCOHOLISM_ALERT.is_active() {
        return draw_alert(
            display,
            &ALCOHOLISM_ALERT,
            ["ALCOHOL", "DEPENDENCE"],
            "Give rehab soon",
        );
    }

    if pet_select::is_active() {
        return pet_select::draw(display);
    }
    if tictactoe::is_active() {
        return tictactoe::draw(display);
    }
    if lightsout::is_active() {
        return lightsout::draw(display);
    }
    if blackhole::is_active() {
        return blackhole::draw(display);
    }
    if nim::is_active() {
        return nim::draw(display);
    }
    if bornjeweled::is_active() {
        return bornjeweled::draw(display);
    }

    // Battery — a slim segmented bar flush against the right edge, not the
    // full icon. The 24 px-wide icon crowded the game menu icon on this
    // screen; the bar is 4 px, leaving that row clear.
    #[cfg(feature = "embassy-core")]
    {
        let pct = crate::fw::battery::read_pct();
        let x = 152 - crate::BATTERY_BAR_W as i32;
        crate::draw_battery_bar(display, x, 2, pct)?;
    }

    // HAX balance — top-right, below the battery icon + menu icon row
    // (y=33 clears the icons; y=16 overlapped them). Larger bold font.
    // Hidden entirely when money mode is disabled for this pet.
    if lifecycle::money_enabled() {
        let right_style = crate::ui::TOP_RIGHT;
        let mut money_buf: heapless::String<12> = heapless::String::new();
        let _ = core::fmt::Write::write_fmt(
            &mut money_buf,
            format_args!("{} H", lifecycle::money()),
        );
        // Reuse the already-linked bold font (FONT_7X13_BOLD via `font`) —
        // pulling in a second glyph table (9x15) overflowed FLASH by ~12 KB.
        Text::with_text_style(money_buf.as_str(), HAX_POS, font, right_style)
            .draw(display)?;
    }

    // Sim-only sprite blit: in firmware the async `render()` blits sprites
    // from FAT12 between display refreshes; the simulator has no flash, so
    // we resolve PCX assets directly from `assets/to-badge/` here.  Drawn
    // before icons/modals so the UI overlays it the same way it would on
    // hardware.  Sprite frame is driven from wall-clock elapsed time so
    // multi-frame animations actually cycle.
    #[cfg(feature = "simulator")]
    {
        use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

        use engine::anim_files;
        // Per-animation start delta: when the anim id changes we pin
        // the wall-clock origin so the new anim begins at frame 0.
        // The free-running clock keeps ticking — we just subtract.
        static LAST_ANIM_ID: AtomicU8 = AtomicU8::new(0xFF);
        static ANIM_START_MS: AtomicU64 = AtomicU64::new(0);
        // Frame interval fixed when the animation starts, so a shrinking
        // action/hatching countdown cannot make the frame index jump or wrap.
        static ANIM_INTERVAL_MS: AtomicU64 = AtomicU64::new(u64::MAX);

        if !lifecycle::is_started() {
            sprite_loader::blit_pcx_sim(display, &anim_files::start_screen_filename(), 0, 0, false);
        } else {
            let kind = lifecycle::pet_kind();
            let anim = lifecycle::display_anim();
            let count = anim_files::frame_count(kind, anim);
            if count > 0 {
                let elapsed_ms = crate::platform::uptime_ms();
                let id = anim_files::anim_id_for(anim);
                if LAST_ANIM_ID.load(Ordering::Relaxed) != id {
                    LAST_ANIM_ID.store(id, Ordering::Relaxed);
                    ANIM_START_MS.store(elapsed_ms, Ordering::Relaxed);
                    let interval = pacing::frame_interval_secs(
                        anim,
                        count as u32,
                        lifecycle::action_ticks_remaining(),
                    )
                    .map_or(u64::MAX, |s| s.saturating_mul(1000));
                    ANIM_INTERVAL_MS.store(interval, Ordering::Relaxed);
                }
                let delta_ms = elapsed_ms.saturating_sub(ANIM_START_MS.load(Ordering::Relaxed));
                // Same pace as the firmware waiter (`pacing`): idle frames
                // every 5 minutes, actions and hatching at full speed.
                let interval_ms = ANIM_INTERVAL_MS.load(Ordering::Relaxed);
                let raw = (delta_ms / interval_ms).min(u32::MAX as u64) as u32;
                let frame = anim_files::clamp_or_wrap(raw, count, anim);
                let name = anim_files::anim_filename(kind, anim, frame);
                sprite_loader::blit_pcx_sim(display, &name, 0, PET_AREA_TOP as i32, false);
            }
        }
    }

    // ── Not started ──────────────────────────────────────────────────────
    if !lifecycle::is_started() {
        // Start screen graphic is blitted by render(); only battery shown here.
        return Ok(());
    }

    let anim = lifecycle::display_anim();

    // ── Hatching ─────────────────────────────────────────────────────────
    if let DisplayAnim::Hatching { ticks_remaining } = anim {
        // Egg animation is blitted by embassy.rs.  The countdown timer
        // below acts as the sole hatching indicator.
        let secs = ticks_remaining as u32 * 10;
        let mut time_str: heapless::String<16> = heapless::String::new();
        let _ = core::fmt::Write::write_fmt(
            &mut time_str,
            format_args!("{}:{:02}", secs / 60, secs % 60),
        );
        Text::with_text_style(time_str.as_str(), Point::new(76, 100), font, centered)
            .draw(display)?;
        return Ok(());
    }

    // ── Gone ─────────────────────────────────────────────────────────────
    if anim == DisplayAnim::Gone {
        // A Gone pet shows no menu or live view.
        if modal::is_open() {
            modal::close();
        }
        // Farewell animation blitted by embassy.rs if available.
        if sprite_loader::frame_count() == 0 {
            Text::with_text_style("Your pet has left", Point::new(76, 50), font_red, centered)
                .draw(display)?;
        }
        Text::with_text_style("Press Fire", Point::new(76, 90), font_red, centered).draw(display)?;
        Text::with_text_style("for a new egg", Point::new(76, 106), font_red, centered)
            .draw(display)?;
        return Ok(());
    }

    // ── Active game ──────────────────────────────────────────────────────

    // Menu icons — sprite-based.  In firmware the async `render()`
    // pre-pass already blitted the six 26×26 PCX icons before this
    // function ran (one per slot, F1 = normal, F2 = selected, the
    // selected variant replaces the firmware-drawn selection circle
    // entirely).  In simulator we don't have an async pre-pass, so
    // resolve them here from `assets/to-badge/`.
    #[cfg(feature = "simulator")]
    for slot in 0..engine::anim_files::MENU_ICON_COUNT {
        let (top_row, col) = engine::anim_files::menu_icon_pos(slot);
        let row_kind = if top_row { Row::Top } else { Row::Bottom };
        let cy = if matches!(row_kind, Row::Top) {
            TOP_CY
        } else {
            BOT_CY
        };
        let cx = ICON_CX[col as usize];
        let selected = nav.row == row_kind && nav.col == col;
        let name = engine::anim_files::menu_icon_filename(slot, selected);
        sprite_loader::blit_pcx_sim(display, &name, cx - 13, cy - 13, false);
    }

    Rectangle::new(Point::new(0, SEP_TOP), Size::new(152, 1))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(display)?;

    // Pet area: sprite blitted by embassy.rs, or fallback.
    if sprite_loader::frame_count() == 0 {
        Text::with_text_style(
            "No sprites on flash",
            Point::new(76, (SEP_TOP + SEP_BOT) / 2),
            font,
            centered,
        )
        .draw(display)?;
    }

    Rectangle::new(Point::new(0, SEP_BOT), Size::new(152, 1))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(display)?;

    // Action feedback toast — shown until the first render at or
    // after TOAST_MIN_VISIBLE_MS has elapsed since show_toast.
    if TOAST.is_active() {
        let toast = Toast::from_u8(TOAST_MSG.load(Ordering::Relaxed));
        // Dynamic toasts (station cooldown) format their text at draw
        // time from a small atomic; everything else uses the static
        // message table.
        let mut dyn_buf: heapless::String<48> = heapless::String::new();
        let msg: &str = if let Toast::Reason = toast {
            let why = engine::doses::Reason::unpack(
                REASON_KIND.load(Ordering::Relaxed),
                REASON_SECS.load(Ordering::Relaxed),
            );
            dyn_buf.push_str(why.text().as_str()).ok();
            dyn_buf.as_str()
        } else if let Toast::DebugInfo = toast {
            // Read live: the toast shows what the engine is doing now.
            if let Some(info) = lifecycle::debug_info() {
                let line = info.line();
                dyn_buf.push_str(line.as_str()).ok();
            }
            dyn_buf.as_str()
        } else if let Toast::StationCooldown = toast {
            let secs = STATION_COOLDOWN_SECS.load(Ordering::Relaxed);
            let m = secs / 60;
            let s = secs % 60;
            let _ = core::fmt::Write::write_fmt(&mut dyn_buf, format_args!("wait {}:{:02}", m, s));
            dyn_buf.as_str()
        } else {
            toast.message()
        };
        if let Toast::Cause = toast {
            draw_cause_toast(display)?;
        } else if !msg.is_empty() {
            
            
            use embedded_graphics::text::{Text};
            let style = crate::ui::TOP_LEFT;
            Text::with_text_style(
                msg,
                TOAST_POS,
                // Red so the action feedback (-hunger / -sick / +inspired /
                // bonus) pops against the black UI.
                crate::ui::TEXT_BOLD_RED,
                style,
            )
            .draw(display)?;
        }
        TOAST.clear_if_elapsed(TOAST_MIN_VISIBLE_MS);
    }

    // The status line replaces the old corner banners. A hibernating pet is
    // frozen, so it says nothing.
    if !lifecycle::is_hibernating()
        && let Some(line) = lifecycle::status_line()
    {
        draw_status_line(display, &line)?;
    }

    modal::draw_modal(display)?;

    Ok(())
}

// ── Async render — called from the display loop ──────────────────────────────

/// Full game render cycle: engine update, sprite blit, debug overlay, save.
///
/// Handles the start screen (full 152×152 blit of `00000000.PCX`),
/// in-game animation blitting, and the debug animation name overlay
/// when no artwork is loaded.
#[cfg(feature = "embassy-core")]
pub async fn render<D>(display: &mut D, sprite_frame: u8)
where
    D: embedded_graphics::draw_target::DrawTarget<Color = crate::TriColor>
        + crate::epd_driver::PlaneAccess,
{
    use engine::anim_files;
    use engine::to_display::DisplayAnim;

    use crate::fw::fat12;

    if lifecycle::is_started() {
        lifecycle::cycle();

        // After hatching completes, prompt the player to name their pet.
        if lifecycle::take_naming_pending() {
            let seed = lifecycle::now_tick();
            let default = lifecycle::random_default_name(seed);
            crate::text_entry::begin(default.as_bytes(), 12, on_pet_named, "Name your Pet", true);
        }
    }

    // Blit sprite from flash.
    let mut has_sprite = false;

    if !lifecycle::is_started() {
        // Start screen: full 152×152 graphic at origin.
        let start_name = anim_files::start_screen_filename();
        if let Ok(file) = fat12::find_file(&start_name).await {
            sprite_loader::blit_file(display, &file, 0, 0, false).await;
            has_sprite = true;
        }
    } else {
        // In-game animation in the pet area.
        let kind = lifecycle::pet_kind();
        let anim = lifecycle::display_anim();
        let frame_count = anim_files::frame_count(kind, anim);
        if frame_count > 0 {
            let name = anim_files::anim_filename(kind, anim, sprite_frame);
            if let Ok(file) = fat12::find_file(&name).await {
                sprite_loader::blit_file(display, &file, 0, PET_AREA_TOP as i32, false).await;
                has_sprite = true;
            }
        }

        // Menu icons (top + bottom rows) — six 26×26 PCX sprites under
        // prefix `0x03`.  The selected variant fully replaces the
        // firmware-drawn selection circle so we don't call
        // `draw_selection_bg` for those slots.  Missing PCX files fail
        // soft (cell stays whatever the EPD was cleared to).
        let nav = nav::get_nav();
        for slot in 0..anim_files::MENU_ICON_COUNT {
            let (top_row, col) = anim_files::menu_icon_pos(slot);
            let cy = if top_row { TOP_CY } else { BOT_CY };
            let cx = ICON_CX[col as usize];
            let row_kind = if top_row { Row::Top } else { Row::Bottom };
            let selected = nav.row == row_kind && nav.col == col;
            let name = anim_files::menu_icon_filename(slot, selected);
            if let Ok(file) = fat12::find_file(&name).await {
                sprite_loader::blit_file(display, &file, cx - 13, cy - 13, false).await;
            }
        }
    }

    // Debug: show animation name when no artwork loaded.
    if !has_sprite && lifecycle::is_started() {
        use embedded_graphics::text::{Text};

        let anim = lifecycle::display_anim();
        let anim_name: &str = match anim {
            DisplayAnim::Gone => "GONE",
            DisplayAnim::Hibernating => "HIBERNATE",
            DisplayAnim::Hatching { .. } => "HATCHING",
            DisplayAnim::Feeding => "FEEDING",
            DisplayAnim::Healing => "HEALING",
            DisplayAnim::Playing => "PLAYING",
            DisplayAnim::Sleeping => "SLEEPING",
            DisplayAnim::Exercising => "EXERCISING",
            DisplayAnim::Medicating => "MEDICATING",
            DisplayAnim::Drinking => "DRINKING",
            DisplayAnim::Ozempic => "OZEMPIC",
            DisplayAnim::Rehab => "REHAB",
            DisplayAnim::OnlyPets => "ONLYPETS",
            DisplayAnim::Leaving { .. } => "LEAVING",
            DisplayAnim::CriticalSick => "CRIT:SICK",
            DisplayAnim::CriticalTired => "CRIT:TIRED",
            DisplayAnim::CriticalHungry => "CRIT:HUNGRY",
            DisplayAnim::WarningSick => "WARN:SICK",
            DisplayAnim::WarningTired => "WARN:TIRED",
            DisplayAnim::WarningHungry => "WARN:HUNGRY",
            DisplayAnim::WarningMiserable => "WARN:MISER",
            DisplayAnim::Happy => "HAPPY",
            DisplayAnim::Idle => "IDLE",
        };
        let style = crate::ui::TOP_RIGHT;
        let _ = Text::with_text_style(
            anim_name,
            Point::new(150, 36),
            crate::ui::TEXT_BLACK,
            style,
        )
        .draw(display);
    }

    if lifecycle::is_started() {
        lifecycle::save_if_needed().await;
    }
}

/// Text entry callback: player has submitted a pet name.
#[cfg(feature = "embassy-core")]
fn on_pet_named(name: &[u8]) {
    lifecycle::set_pet_name(name);
}

#[cfg(test)]
mod battle_anim_tests {
    use super::{BattleStage, battle_stage_from_elapsed};

    #[test]
    fn stage_boundaries() {
        assert_eq!(battle_stage_from_elapsed(0), BattleStage::Standing);
        assert_eq!(battle_stage_from_elapsed(4_999), BattleStage::Standing);
        assert_eq!(battle_stage_from_elapsed(5_000), BattleStage::Result);
        assert_eq!(battle_stage_from_elapsed(9_999), BattleStage::Result);
        assert_eq!(battle_stage_from_elapsed(10_000), BattleStage::Done);
        assert_eq!(battle_stage_from_elapsed(u32::MAX), BattleStage::Done);
    }
}

/// The one lock for every test that touches game-global state (`GAME`, the
/// toast, the sleep-animation floor, `battle_view`, alerts). Hold the guard
/// for the whole test; dropping it (also on a failed assert) clears the UI
/// statics so the next test starts clean.
#[cfg(test)]
pub(crate) struct TestState(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

#[cfg(test)]
static TEST_STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn lock_test_state() -> TestState {
    TestState(TEST_STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
impl Drop for TestState {
    fn drop(&mut self) {
        TOAST.clear();
        DIABETES_ALERT.clear();
        ALCOHOLISM_ALERT.clear();
        lifecycle::clear_sleep_anim_for_test();
        battle_view::reset_for_test();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_cause_toast_clears_the_hax_balance() {
        use embedded_graphics::prelude::*;
        use embedded_graphics::text::Text;
        // The widest balance the badge shows: "{} H" of a six-digit number.
        let mut hax: heapless::String<12> = heapless::String::new();
        core::fmt::Write::write_fmt(&mut hax, format_args!("{} H", 999_999)).unwrap();
        let hax_box = Text::with_text_style(
            hax.as_str(),
            super::HAX_POS,
            crate::ui::TEXT_BOLD_BLACK,
            crate::ui::TOP_RIGHT,
        )
        .bounding_box();
        for cause in super::engine::causes::Cause::all() {
            let what_box = Text::with_text_style(
                cause.what(),
                super::TOAST_POS,
                crate::ui::TEXT_BOLD_RED,
                crate::ui::TOP_LEFT,
            )
            .bounding_box();
            assert!(
                what_box.intersection(&hax_box).is_zero_sized(),
                "{cause:?}: {what_box:?} meets {hax_box:?}"
            );
            let why_box = Text::with_text_style(
                cause.why(),
                super::TOAST_POS + Point::new(0, super::TOAST_LINE_DY),
                crate::ui::TEXT_RED,
                crate::ui::TOP_LEFT,
            )
            .bounding_box();
            assert!(why_box.intersection(&hax_box).is_zero_sized(), "{cause:?}");
            assert!(why_box.bottom_right().unwrap().x <= 150, "{cause:?}");
        }
    }

    #[test]
    fn toasts_name_the_v3_effects() {
        use super::Toast;
        for (toast, text) in [
            (Toast::Feed, "+full"),
            (Toast::Heal, "healing"),
            (Toast::Sleep, "sleeping"),
            (Toast::Relax, "calmer +energy"),
            (Toast::Wash, "cleaner"),
            (Toast::Play, "+fun"),
            (Toast::Rehab, "rehab started"),
            (Toast::Ozempic, "-weight"),
        ] {
            assert_eq!(toast.message(), text);
            assert!(
                text.len() <= 14,
                "{text}: wider than the longest toast (Inspire bonus!)"
            );
        }
    }

    use super::test_canvas::Canvas;
    use super::{SEP_BOT, draw_status_line};
    use crate::game::engine::status::{StatusLine, Word};
    use crate::{BLACK, RED};
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::Rectangle;

    fn rows(y0: i32, y1: i32) -> Rectangle {
        Rectangle::new(Point::new(0, y0), Size::new(152, (y1 - y0 + 1) as u32))
    }

    /// Clears the installed test pet on drop, also after a failed assert.
    struct ClearPet;
    impl Drop for ClearPet {
        fn drop(&mut self) {
            crate::game::lifecycle::set_state_for_test(None);
        }
    }

    fn line_of(first: Word, danger: bool) -> StatusLine {
        StatusLine {
            first,
            second: None,
            danger,
        }
    }

    /// A one-line status line drawn over a `ink` backdrop (the sprite): the
    /// backing strip around the glyphs must be white, the rest untouched.
    fn assert_backing(first: Word, danger: bool, ink: crate::TriColor) {
        let mut c = Canvas::new();
        c.fill(ink);
        draw_status_line(&mut c, &line_of(first, danger)).unwrap();
        // Text width + STATUS_BACKING_PAD, centred, rows 97..=109.
        let w = first.text().len() as i32 * super::STATUS_CHAR_W + super::STATUS_BACKING_PAD;
        let x0 = super::STATUS_CENTER_X - w / 2;
        let y0 = super::STATUS_BASE_Y - (super::STATUS_LINE_H - 1);
        let pad = super::STATUS_BACKING_PAD / 2;
        for y in y0..=super::STATUS_BASE_Y {
            for x in (x0..x0 + pad).chain(x0 + w - pad..x0 + w) {
                assert_eq!(c.get(x, y), crate::WHITE, "backing margin at ({x},{y})");
            }
        }
        for x in x0..x0 + w {
            assert_eq!(c.get(x, y0), crate::WHITE, "backing top at x={x}");
        }
        assert_eq!(c.get(x0 - 1, y0 + 5), ink, "backing is no wider");
        assert_eq!(c.get(x0 + w, y0 + 5), ink, "backing is no wider");
        assert_eq!(c.get(x0 + 10, y0 - 1), ink, "backing is no taller");
    }

    #[test]
    fn status_line_draws_at_the_pets_feet() {
        assert_backing(Word::Hungry, false, BLACK);
        let mut c = Canvas::new();
        draw_status_line(&mut c, &line_of(Word::Hungry, false)).unwrap();
        assert!(c.any(rows(97, 109), BLACK));
        assert!(!c.any(rows(0, 96), BLACK));
        assert!(!c.any(rows(0, 151), RED));
        assert!(!c.any(rows(SEP_BOT - 1, 151), BLACK));
    }

    #[test]
    fn danger_line_is_red() {
        assert_backing(Word::Starving, true, BLACK);
        let mut c = Canvas::new();
        draw_status_line(&mut c, &line_of(Word::Starving, true)).unwrap();
        assert!(c.any(rows(97, 109), RED));
        assert!(!c.any(rows(0, 96), RED));
        assert!(!c.any(rows(SEP_BOT - 1, 151), RED));
        assert!(!c.any(rows(0, 151), BLACK));
    }

    #[test]
    fn two_long_words_take_two_lines_inside_the_pet_area() {
        let mut c = Canvas::new();
        let line = StatusLine {
            first: Word::Waterlogged,
            second: Some(Word::Underweight),
            danger: false,
        };
        assert_eq!(line.text().as_str(), "waterlogged\nunderweight");
        draw_status_line(&mut c, &line).unwrap();
        assert!(c.any(rows(84, 96), BLACK));
        assert!(c.any(rows(97, 109), BLACK));
        assert!(!c.any(rows(0, 83), BLACK));
        assert!(!c.any(rows(SEP_BOT - 1, 151), BLACK));
    }

    #[test]
    fn full_refresh_follows_danger_not_banners() {
        use crate::game::engine::needs::P;
        use crate::game::engine::{GameState, PetKind, Phase};
        let _guard = super::lock_test_state();
        let _clear = ClearPet;
        let pet = || {
            let mut s = GameState::new_egg(7, PetKind(0));
            s.phase = Phase::Active;
            s
        };
        // 75 % weight was the old OVERWEIGHT banner: no longer a red message.
        let mut s = pet();
        s.weight = (75 * P) as u16;
        crate::game::lifecycle::set_state_for_test(Some(s));
        assert!(!super::status_wants_full_refresh());
        let mut s = pet();
        s.fullness = (10 * P) as u16;
        crate::game::lifecycle::set_state_for_test(Some(s.clone()));
        assert!(super::status_wants_full_refresh());
        // The line is not drawn while hibernating, so nothing red to seat.
        s.hibernating = true;
        crate::game::lifecycle::set_state_for_test(Some(s));
        assert!(!super::status_wants_full_refresh());
    }

    #[test]
    fn the_battle_result_card_needs_no_wake_loop() {
        let _guard = super::lock_test_state();
        // The card waits for a button press; the press redraws. Polling it
        // every tick would wake the badge for as long as it is left up.
        super::battle_view::show_result_for_test();
        assert!(super::battle_view::wants_full_refresh());
        assert_eq!(super::next_ui_deadline_tick(), u32::MAX);
    }

    #[test]
    fn live_deadline_is_on_a_grid_never_in_the_past() {
        use super::{live_deadline, pacing::IDLE_FRAME_TICKS as F};
        assert_eq!(live_deadline(500, 500), 500 + F);
        assert_eq!(live_deadline(500, 500 + F - 1), 500 + F);
        assert_eq!(live_deadline(500, 500 + F), 500 + 2 * F);
        let now = 1000;
        let d = live_deadline(now - 100, now);
        assert!(d > now && d <= now + F, "{d}");
        // a later draw moves the anchor
        assert_eq!(live_deadline(now, now), now + F);
    }

    #[test]
    fn a_gone_pet_keeps_no_live_view() {
        use crate::game::engine::{GameState, PetKind, Phase};
        let _guard = super::lock_test_state();
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        crate::game::lifecycle::set_state_for_test(Some(s));
        super::modal::open(super::modal::ModalKind::Stats);
        super::modal::activate();
        assert!(super::live_view_open());
        let mut gone = GameState::new_egg(7, PetKind(0));
        gone.phase = Phase::Gone;
        crate::game::lifecycle::set_state_for_test(Some(gone));
        assert_eq!(super::lifecycle::display_anim(), super::engine::DisplayAnim::Gone);
        assert!(!super::live_view_open());
        assert_eq!(super::next_ui_deadline_tick(), u32::MAX);
        super::modal::close();
        crate::game::lifecycle::set_state_for_test(None);
    }

    #[test]
    fn an_open_stats_view_refreshes_within_an_idle_frame() {
        use crate::game::engine::{GameState, PetKind, Phase};
        let _guard = super::lock_test_state();
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        crate::game::lifecycle::set_state_for_test(Some(s));
        assert_eq!(super::next_ui_deadline_tick(), u32::MAX);
        super::modal::open(super::modal::ModalKind::Stats);
        assert_eq!(
            super::next_ui_deadline_tick(),
            u32::MAX,
            "menu list has no drifting numbers"
        );
        super::modal::activate(); // first row: View stats
        assert!(super::live_view_open());
        // The deadline is anchored to the last draw, never to `now`: time
        // passing without a redraw (minute ticks) must not move it.
        super::LIVE_DRAWN_TICK.store(1000, core::sync::atomic::Ordering::Relaxed);
        let anchored = 1000 + super::pacing::IDLE_FRAME_TICKS;
        assert_eq!(super::next_ui_deadline_tick(), anchored);
        assert_eq!(super::next_ui_deadline_tick(), anchored);
        super::modal::close();
        assert!(!super::live_view_open());
        assert_eq!(super::next_ui_deadline_tick(), u32::MAX);
        crate::game::lifecycle::set_state_for_test(None);
    }
}
