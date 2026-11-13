//! Diary view: the last cause events, three per page, two pages.
//!
//! A full-screen overlay opened from the Stats menu. Left / Right flip the
//! page, any other button closes it. The ages change by themselves, so an
//! open diary is a live view (see `game::live_view_open`).

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use embedded_graphics::prelude::*;

use super::diary::{DIARY_LEN, entry_lines};
use crate::TriColor;
use crate::ui::{self, TEXT_BLACK, TEXT_BOLD_BLACK};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static PAGE: AtomicU8 = AtomicU8::new(0);

/// Entries per page.
const PER_PAGE: usize = 3;
const PAGES: u8 = DIARY_LEN.div_ceil(PER_PAGE) as u8;
/// Y of the first entry's first line, and the distance between entries.
const FIRST_ROW_Y: i32 = 22;
const ROW_PITCH: i32 = 40;
/// How far the second line sits below the first.
const LINE2_DY: i32 = 13;
const LEFT_X: i32 = 4;

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn open() {
    PAGE.store(0, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
    super::note_live_view_drawn();
}

pub fn close() {
    ACTIVE.store(false, Ordering::Relaxed);
}

/// Show the other page (wraps).
pub fn flip() {
    let next = (PAGE.load(Ordering::Relaxed) + 1) % PAGES;
    PAGE.store(next, Ordering::Relaxed);
}

pub fn page() -> u8 {
    PAGE.load(Ordering::Relaxed)
}

pub fn draw<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    super::note_live_view_drawn();
    ui::clear_screen(display)?;

    let page = page();
    let mut title: heapless::String<12> = heapless::String::new();
    let _ = core::fmt::Write::write_fmt(&mut title, format_args!("Diary {}/{}", page + 1, PAGES));
    ui::draw_title_bar(display, title.as_str(), Point::zero(), 152)?;

    let diary = super::lifecycle::diary();
    if diary.is_empty() {
        return ui::draw_centered_message(display, "Nothing yet", Point::new(76, 76));
    }

    let now_min = super::lifecycle::pet_age_min();
    let shown = diary
        .newest_first()
        .skip(page as usize * PER_PAGE)
        .take(PER_PAGE);
    for (i, entry) in shown.enumerate() {
        let y = FIRST_ROW_Y + ROW_PITCH * i as i32;
        let (what, why) = entry_lines(entry, now_min);
        ui::draw_text(
            display,
            what.as_str(),
            Point::new(LEFT_X, y),
            TEXT_BOLD_BLACK,
            ui::TOP_LEFT,
        )?;
        ui::draw_text(
            display,
            why.as_str(),
            Point::new(LEFT_X, y + LINE2_DY),
            TEXT_BLACK,
            ui::TOP_LEFT,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::game::engine::{GameState, PetKind, Phase};
    use crate::menu::ButtonId;

    #[test]
    fn diary_pages_flip_with_left_right() {
        let _guard = crate::game::lock_test_state();
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        crate::game::lifecycle::set_state_for_test(Some(s));
        super::open();
        assert!(super::is_active());
        assert_eq!(super::page(), 0);
        assert!(crate::game::input::dispatch(ButtonId::Right));
        assert_eq!(super::page(), 1);
        assert!(crate::game::input::dispatch(ButtonId::Right));
        assert_eq!(super::page(), 0);
        assert!(crate::game::input::dispatch(ButtonId::Left));
        assert_eq!(super::page(), 1);
        assert!(crate::game::input::dispatch(ButtonId::Fire));
        assert!(!super::is_active());
        super::open();
        assert_eq!(super::page(), 0, "reopening starts at page 1");
        super::close();
        crate::game::lifecycle::set_state_for_test(None);
    }

    #[test]
    fn an_open_diary_refreshes_within_an_idle_frame() {
        let _guard = crate::game::lock_test_state();
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        crate::game::lifecycle::set_state_for_test(Some(s));
        assert_eq!(crate::game::next_ui_deadline_tick(), u32::MAX);
        super::open();
        let now = crate::game::lifecycle::now_tick();
        assert!(crate::game::live_view_open());
        assert!(crate::game::next_ui_deadline_tick() <= now + 30);
        super::close();
        assert_eq!(crate::game::next_ui_deadline_tick(), u32::MAX);
        crate::game::lifecycle::set_state_for_test(None);
    }
}
