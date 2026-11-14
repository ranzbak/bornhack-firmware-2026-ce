//! The pet sprites for the booklet (`guide sprites`): every frame of the
//! animations the guide shows, for every pet in `PETS.CFG`, as a PNG on a
//! white ground, plus `roster.json` listing what exists. Sponsor art (prefix
//! 03) and menu icons (prefix 04) are never in the roster, so never exported.

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::path::Path;

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::SimulatorDisplay;

use super::json::J;
use super::shots::HERO_NAME;
use crate::game::engine::anim_files::{build_filename, frame_count, start_screen_filename};
use crate::game::engine::{DisplayAnim, PetKind};
use crate::game::pet_registry;
use crate::game::sprite_loader::{blit_pcx_sim, pcx_size_sim};
use crate::sim_support::{self, TriColorDisplay};

/// The animations the guide shows, under the names the booklet asks for.
pub const ANIMS: [(&str, DisplayAnim); 15] = [
    ("idle", DisplayAnim::Idle),
    ("happy", DisplayAnim::Happy),
    ("sleeping", DisplayAnim::Sleeping),
    ("feeding", DisplayAnim::Feeding),
    ("drinking", DisplayAnim::Drinking),
    ("playing", DisplayAnim::Playing),
    ("exercising", DisplayAnim::Exercising),
    ("healing", DisplayAnim::Healing),
    ("hungry", DisplayAnim::WarningHungry),
    ("tired", DisplayAnim::WarningTired),
    ("sick", DisplayAnim::WarningSick),
    ("critical", DisplayAnim::CriticalSick),
    ("hibernating", DisplayAnim::Hibernating),
    ("egg", DisplayAnim::Hatching { ticks_remaining: 0 }),
    ("gone", DisplayAnim::Gone),
];

/// The prefix as it appears in file names: two uppercase hex digits.
fn hex(prefix: u8) -> String {
    alloc::format!("{prefix:02X}")
}

/// Draw the PCX `name` on a white display of its own size and save it.
fn save_pcx(name: &[u8; 11], path: &Path) -> Result<(), String> {
    let (w, h) = pcx_size_sim(name).ok_or_else(|| {
        alloc::format!("missing or unreadable sprite {}", String::from_utf8_lossy(name))
    })?;
    let mut display: SimulatorDisplay<Rgb888> = SimulatorDisplay::new(Size::new(w, h));
    display.clear(Rgb888::new(255, 255, 255)).unwrap();
    blit_pcx_sim(&mut TriColorDisplay(&mut display), name, 0, 0, false);
    sim_support::save_png(&display, path)
}

/// Write every sprite PNG into `out_dir` and return the roster JSON.
pub fn export(out_dir: &Path) -> J {
    sim_support::install_pets_from_repo_cfg();
    std::fs::create_dir_all(out_dir).expect("create the sprite folder");
    let mut pets: Vec<J> = Vec::new();
    for &kind in PetKind::roster() {
        let prefix = hex(kind.0);
        let mut anims: Vec<(String, J)> = Vec::new();
        for (name, anim) in ANIMS {
            let frames = frame_count(kind, anim);
            for f in 0..frames {
                let path = out_dir.join(alloc::format!("{prefix}-{name}-{f}.png"));
                if let Err(e) = save_pcx(&build_filename(kind, anim, f), &path) {
                    panic!("{e}");
                }
            }
            if frames > 0 {
                anims.push((name.to_string(), J::Int(frames as i64)));
            }
        }
        pets.push(J::Obj(alloc::vec![
            ("prefix".to_string(), J::Str(prefix)),
            ("name".to_string(), J::Str(pet_registry::name_of(kind.0).to_string())),
            ("anims".to_string(), J::Obj(anims)),
        ]));
    }
    save_pcx(&start_screen_filename(), &out_dir.join("start.png")).expect("the start screen");
    let hero = PetKind::roster().first().map_or(0, |k| k.0);
    J::Obj(alloc::vec![
        (
            "hero".to_string(),
            J::Obj(alloc::vec![
                ("name".to_string(), J::Str(HERO_NAME.to_string())),
                ("prefix".to_string(), J::Str(hex(hero))),
            ]),
        ),
        ("pets".to_string(), J::Arr(pets)),
        ("start".to_string(), J::Str("start.png".to_string())),
    ])
}
