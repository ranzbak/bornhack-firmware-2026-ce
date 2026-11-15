//! Station NFC dispatcher: one tap, one fix, one answer.
//!
//! Two callers reach [`apply`]:
//!   * the signed NFC channel (`nfct::handle_signed`): authenticated, always
//!     on, and may cure the chronic conditions;
//!   * an unsigned plaintext NDEF write (`nfct::handle_ndef_write`): off
//!     unless `nfc-plaintext-station`, and it cannot cure them.
//!
//! The text is a station message (`engine::fix`): `fix s=<id> ...` or an old
//! phrase kept as an alias.  Each station id rests for its own cooldown
//! (default 5 minutes, `cd=` up to 24 h); the table lives in RAM only and
//! forgets at reboot.  A refused tap always says why (a toast), except on a
//! badge with no game at all, where the start screen is the answer.

use super::engine::fix::{self, Fix, STATION_ID_MAX, cond};
use super::engine::{Phase, TICKS_PER_MINUTE, doses::Reason, thresholds as th};
use super::lifecycle::{self, with_game_now};
use super::{StationToast, modal, show_reason, show_station};

/// Stations remembered at once; a ninth evicts the one free soonest.
pub const COOLDOWN_SLOTS: usize = 8;

#[derive(Clone, Copy)]
struct Slot {
    id: [u8; STATION_ID_MAX],
    len: u8,
    /// The tick the station is free again.
    until: u32,
}

impl Slot {
    fn is(&self, id: &[u8]) -> bool {
        &self.id[..self.len as usize] == id
    }
}

/// Per-station cooldowns.
pub struct Cooldowns {
    slots: [Option<Slot>; COOLDOWN_SLOTS],
}

impl Cooldowns {
    pub const fn new() -> Self {
        Self { slots: [None; COOLDOWN_SLOTS] }
    }

    /// Ticks left for `id`, None when free.
    pub fn remaining(&self, id: &[u8], now: u32) -> Option<u32> {
        self.slots
            .iter()
            .flatten()
            .find(|s| s.is(id))
            .and_then(|s| s.until.checked_sub(now))
            .filter(|&t| t > 0)
    }

    /// Remember `id` until now + ticks: its own slot, else an empty or free
    /// slot, else the slot that frees soonest.
    pub fn stamp(&mut self, id: &[u8], now: u32, ticks: u32) {
        let at = self
            .slots
            .iter()
            .position(|s| s.is_some_and(|s| s.is(id)))
            .or_else(|| self.slots.iter().position(|s| s.is_none_or(|s| s.until <= now)))
            .or_else(|| {
                (0..COOLDOWN_SLOTS).min_by_key(|&i| self.slots[i].map_or(0, |s| s.until))
            })
            .unwrap_or(0);
        let mut buf = [0u8; STATION_ID_MAX];
        let len = id.len().min(STATION_ID_MAX);
        buf[..len].copy_from_slice(&id[..len]);
        self.slots[at] = Some(Slot { id: buf, len: len as u8, until: now.saturating_add(ticks) });
    }
}

static COOLDOWNS: crate::sync::SharedCell<Cooldowns> =
    crate::sync::SharedCell::new(Cooldowns::new());

/// clamp(cd.unwrap_or(STATION_CD_MIN), STATION_CD_MIN, STATION_CD_MAX_MIN) minutes, in ticks.
pub fn cooldown_ticks(cd_min: Option<u16>) -> u32 {
    let lo = th::STATION_CD_MIN();
    let hi = th::STATION_CD_MAX_MIN();
    cd_min.unwrap_or(lo).clamp(lo, hi) as u32 * TICKS_PER_MINUTE
}

/// What a tap came to.  Its status word goes back over the signed channel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum Outcome {
    /// The fix was applied.
    Applied,
    /// Understood, but on cooldown, pet gone, or no game.
    Refused,
    /// Unreadable, malformed, or nothing known (a station message that failed,
    /// or any unreadable text on the signed path).
    Unknown,
    /// Plain text that is not a station message.
    NotStation,
}

impl Outcome {
    pub fn sw(self) -> [u8; 2] {
        match self {
            Outcome::Applied => [0x90, 0x00],
            Outcome::Refused => [0x69, 0x85],
            Outcome::Unknown | Outcome::NotStation => [0x6A, 0x82],
        }
    }

    /// The text was a station message: the plain NDEF path keeps it transient.
    pub fn is_station(self) -> bool {
        self != Outcome::NotStation
    }
}

/// Put the game screen in front and close an open menu.
fn forward() {
    super::bring_game_forward();
    if modal::is_open() {
        modal::close();
    }
}

fn unknown() -> Outcome {
    show_station(StationToast::Unknown, b"");
    forward();
    Outcome::Unknown
}

/// One tap.  `signed`: it came through the verified signed channel.
pub fn apply(text: &[u8], signed: bool) -> Outcome {
    let mut fix: Fix = match fix::parse_text(text) {
        Ok(Some(f)) => f,
        Ok(None) if !signed => return Outcome::NotStation,
        Ok(None) | Err(_) => return unknown(),
    };
    if !signed {
        fix.clear &= !cond::CHRONIC;
        if !fix.understood() {
            return unknown();
        }
    }
    match lifecycle::game_phase() {
        None => {
            forward();
            return Outcome::Refused;
        }
        Some(Phase::Gone) => {
            show_reason(Reason::Gone);
            forward();
            return Outcome::Refused;
        }
        Some(_) => {}
    }
    let now = lifecycle::now_tick();
    if let Some(t) = COOLDOWNS.with(|c| c.remaining(fix.id(), now)) {
        show_station(StationToast::TooSoon(t * 10), fix.id());
        forward();
        return Outcome::Refused;
    }
    let (mut thanks, mut hard) = (false, false);
    with_game_now(|s| {
        thanks = s.station_fix(&fix);
        hard = s.hard_mode;
        s.request_save();
    });
    COOLDOWNS.with_mut(|c| c.stamp(fix.id(), now, cooldown_ticks(fix.cd_min)));
    let toast = if hard {
        StationToast::Covered
    } else if thanks {
        StationToast::Thanks
    } else {
        StationToast::Fixed
    };
    show_station(toast, fix.id());
    forward();
    Outcome::Applied
}

/// Forget every cooldown.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    COOLDOWNS.with_mut(|c| *c = Cooldowns::new());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::engine::{GameState, PetKind, Phase, needs::P};
    use crate::game::{StationToast as T, last_station_toast_for_test, lock_test_state};

    const MIN_MS: u64 = 60_000;

    fn pet(phase: Phase) -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = phase;
        s
    }

    fn start(s: GameState) -> crate::game::TestState {
        let g = lock_test_state();
        reset_for_test();
        crate::platform::set_fake_uptime_ms(0);
        lifecycle::set_state_for_test(Some(s));
        crate::game::modal::close();
        crate::DISPLAY_STATE.with_mut(|d| d.set_active_screen(crate::SCREEN_MAIN));
        g
    }

    fn at_ms(ms: u64) {
        crate::platform::set_fake_uptime_ms(ms);
    }

    fn read<R: Default>(f: impl FnOnce(&GameState) -> R) -> R {
        let mut out = R::default();
        lifecycle::with_state(|s| {
            out = f(s);
            false
        });
        out
    }

    fn screen() -> u8 {
        crate::DISPLAY_STATE.with(|d| d.active_screen())
    }

    const FOOD: &[u8] = b"fix s=food1 full>60 water>60";
    const PHARM: &[u8] = b"fix s=pharm1 health>100 booze<20 clear=hangover,injury,groggy";
    const CLINIC: &[u8] = b"fix s=clinic1 clear=dependence,diabetes booze<5 fit>50 cd=1440";

    #[test]
    fn an_alias_applies_and_says_where() {
        let mut s = pet(Phase::Active);
        s.fullness = 20 * P as u16;
        let _g = start(s);
        assert_eq!(apply(b"more food", true), Outcome::Applied);
        assert!(read(|s| s.fullness) >= 60 * P as u16);
        let (t, id) = last_station_toast_for_test().unwrap();
        assert_eq!((t, id.as_slice()), (T::Fixed, b"food".as_slice()));
        assert_eq!(screen(), crate::SCREEN_GAME);
    }

    #[test]
    fn the_same_station_rests_and_says_how_long() {
        let _g = start(pet(Phase::Active));
        assert_eq!(apply(FOOD, true), Outcome::Applied);
        assert_eq!(apply(FOOD, true), Outcome::Refused);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::TooSoon(300));
        at_ms(MIN_MS);
        assert_eq!(apply(FOOD, true), Outcome::Refused);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::TooSoon(240));
        at_ms(5 * MIN_MS);
        assert_eq!(apply(FOOD, true), Outcome::Applied);
    }

    #[test]
    fn two_stations_back_to_back() {
        let _g = start(pet(Phase::Active));
        assert_eq!(apply(FOOD, true), Outcome::Applied);
        assert_eq!(apply(PHARM, true), Outcome::Applied);
    }

    #[test]
    fn a_refused_tap_does_not_extend_the_wait() {
        let _g = start(pet(Phase::Active));
        assert_eq!(apply(FOOD, true), Outcome::Applied);
        at_ms(100_000);
        assert_eq!(apply(FOOD, true), Outcome::Refused);
        at_ms(300_000);
        assert_eq!(apply(FOOD, true), Outcome::Applied);
    }

    #[test]
    fn cd_is_clamped() {
        assert_eq!(cooldown_ticks(None), 30);
        assert_eq!(cooldown_ticks(Some(0)), 30);
        assert_eq!(cooldown_ticks(Some(240)), 1440);
        assert_eq!(cooldown_ticks(Some(9999)), 8640);
        let _g = start(pet(Phase::Active));
        let gym = b"fix s=gym1 fit+5 strain<20 cd=240";
        assert_eq!(apply(gym, true), Outcome::Applied);
        at_ms(180 * MIN_MS);
        assert_eq!(apply(gym, true), Outcome::Refused);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::TooSoon(3600));
    }

    #[test]
    fn the_ninth_station_evicts_the_one_free_soonest() {
        let ids: [&[u8]; 9] = [b"s1", b"s2", b"s3", b"s4", b"s5", b"s6", b"s7", b"s8", b"s9"];
        let mut c = Cooldowns::new();
        for (i, id) in ids.iter().take(8).enumerate() {
            c.stamp(id, 0, (10 + i as u32) * 6);
        }
        c.stamp(ids[8], 0, 60);
        assert_eq!(c.remaining(ids[0], 0), None);
        for id in &ids[1..] {
            assert!(c.remaining(id, 0).is_some());
        }

        // s4 is free by the time s9 arrives: its slot goes, s1 stays.
        let mut c = Cooldowns::new();
        for (i, id) in ids.iter().take(8).enumerate() {
            let min = if i == 3 { 1 } else { 10 + i as u32 };
            c.stamp(id, 0, min * 6);
        }
        c.stamp(ids[8], 12, 60);
        assert!(c.remaining(ids[0], 12).is_some());
        assert_eq!(c.remaining(ids[3], 12), None);
        assert!(c.remaining(ids[8], 12).is_some());
        // Stamping an id again reuses its own slot.
        c.stamp(ids[8], 12, 600);
        assert_eq!(c.remaining(ids[8], 12), Some(600));
        assert!(c.remaining(ids[0], 12).is_some());
    }

    #[test]
    fn unknown_text_on_each_path() {
        let mut s = pet(Phase::Active);
        s.fullness = 50 * P as u16;
        let _g = start(s);
        assert_eq!(apply(b"hello", true), Outcome::Unknown);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Unknown);
        assert_eq!(screen(), crate::SCREEN_GAME);

        crate::DISPLAY_STATE.with_mut(|d| d.set_active_screen(crate::SCREEN_MAIN));
        crate::game::show_station(T::Fixed, b"keep");
        assert_eq!(apply(b"hello", false), Outcome::NotStation);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Fixed, "no toast");
        assert_eq!(screen(), crate::SCREEN_MAIN);

        assert_eq!(apply(b"fix s=x full>101", false), Outcome::Unknown);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Unknown);
        assert_eq!(read(|s| s.fullness), 50 * P as u16);
    }

    #[test]
    fn every_refusal_raises_a_toast() {
        let _g = start(pet(Phase::Gone));
        crate::game::TOAST.clear();
        assert_eq!(apply(FOOD, true), Outcome::Refused);
        assert!(crate::game::TOAST.is_active());
        assert_eq!(crate::game::TOAST_MSG.load(core::sync::atomic::Ordering::Relaxed), crate::game::Toast::Reason as u8);
        assert_eq!(screen(), crate::SCREEN_GAME);

        lifecycle::set_state_for_test(Some(pet(Phase::Active)));
        assert_eq!(apply(FOOD, true), Outcome::Applied);
        assert_eq!(apply(FOOD, true), Outcome::Refused);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::TooSoon(300));

        lifecycle::set_state_for_test(None);
        crate::DISPLAY_STATE.with_mut(|d| d.set_active_screen(crate::SCREEN_MAIN));
        crate::game::show_station(T::Fixed, b"keep");
        assert_eq!(apply(b"fix s=zz1 full>60", true), Outcome::Refused);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Fixed, "no toast");
        assert_eq!(screen(), crate::SCREEN_GAME);
    }

    #[test]
    fn an_egg_takes_the_fix() {
        let mut s = pet(Phase::Hatching);
        s.fullness = 10 * P as u16;
        let _g = start(s);
        assert_eq!(apply(FOOD, true), Outcome::Applied);
        assert!(read(|s| s.fullness) >= 60 * P as u16);
    }

    #[test]
    fn the_plain_path_cannot_cure() {
        let mut s = pet(Phase::Active);
        s.dependent = true;
        s.diabetic = true;
        let _g = start(s);
        assert_eq!(apply(CLINIC, false), Outcome::Applied);
        assert!(read(|s| s.dependent && s.diabetic));
        assert_eq!(apply(b"fix s=c2 clear=diabetes", false), Outcome::Unknown);
        assert_eq!(apply(CLINIC, true), Outcome::Refused, "same station rests");
        assert_eq!(apply(b"fix s=c3 clear=dependence,diabetes", true), Outcome::Applied);
        assert!(read(|s| !s.dependent && !s.diabetic));
    }

    #[test]
    fn hard_mode_is_covered_and_demo_says_thanks() {
        let mut s = pet(Phase::Active);
        s.hard_mode = true;
        let _g = start(s);
        assert_eq!(apply(b"fix s=a1 full>60", true), Outcome::Applied);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Covered);

        let mut s = pet(Phase::Active);
        s.demo.on = true;
        lifecycle::set_state_for_test(Some(s));
        assert_eq!(apply(b"fix s=a2 full>60", true), Outcome::Applied);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Thanks);

        lifecycle::set_state_for_test(Some(pet(Phase::Active)));
        assert_eq!(apply(b"fix s=a3 full>60", true), Outcome::Applied);
        assert_eq!(last_station_toast_for_test().unwrap().0, T::Fixed);
    }

    #[test]
    fn a_tap_closes_the_menu() {
        let _g = start(pet(Phase::Active));
        crate::game::modal::open(crate::game::modal::ModalKind::Stats);
        assert!(crate::game::modal::is_open());
        assert_eq!(apply(FOOD, true), Outcome::Applied);
        assert!(!crate::game::modal::is_open());
    }

    #[test]
    fn outcome_status_words() {
        assert_eq!(Outcome::Applied.sw(), [0x90, 0x00]);
        assert_eq!(Outcome::Refused.sw(), [0x69, 0x85]);
        assert_eq!(Outcome::Unknown.sw(), [0x6A, 0x82]);
        assert_eq!(Outcome::NotStation.sw(), [0x6A, 0x82]);
    }

    #[test]
    fn station_outcomes_are_transient_on_the_plain_path() {
        assert!(Outcome::Applied.is_station());
        assert!(Outcome::Refused.is_station());
        assert!(Outcome::Unknown.is_station());
        assert!(!Outcome::NotStation.is_station());
        let _g = start(pet(Phase::Active));
        assert_eq!(apply(FOOD, false), Outcome::Applied);
        assert_eq!(apply(FOOD, false), Outcome::Refused);
    }
}
