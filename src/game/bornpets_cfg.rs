//! Read `BORNPETS.CFG` from the USB-MSC FAT12 partition and apply its
//! overrides on top of the preset selected via the on-badge menu.
//!
//! File format — one `KEY=VALUE` per line, `#` introduces a comment.
//! Whitespace around the `=` is ignored.  Unknown keys are logged and
//! skipped, so adding a new tunable later doesn't break older files.
//!
//! ```text
//! # make food fill more, halve the starvation damage
//! FEED_HUNGER_RELIEF=4000
//! DMG_FULL_LOW_PM=20
//! ```
//!
//! Call [`load_and_install`] exactly once during boot, after `kv::init()`
//! and after `fat12::format_if_needed()`, but **before** the game engine
//! creates any [`crate::game::engine::GameState`].

#[cfg(feature = "embassy-core")]
use crate::game::engine::thresholds;
use crate::game::engine::thresholds::{BORNPETS_CFG_KEYS, Mode, Thresholds};

#[cfg(feature = "embassy-core")]
const READ_BUF_LEN: usize = 2048;

/// Pick the active preset, optionally overlay `BORNPETS.CFG`, install
/// the result into the threshold accessor.  Returns the [`Mode`] used
/// and whether at least one valid override row was applied.
#[cfg(feature = "embassy-core")]
pub async fn load_and_install(mode: Mode) -> (Mode, bool) {
    use crate::fw::fat12;

    let mut values = mode.preset();

    let mut buf = [0u8; READ_BUF_LEN];
    let Ok(n) = fat12::read_named("BORNPETS.CFG", &mut buf).await else {
        thresholds::install(values, mode, false);
        return (mode, false);
    };

    let applied = apply_overrides(&mut values, &buf[..n]);
    let values = thresholds::validate(&values, &mode.preset());
    defmt::info!(
        "BORNPETS.CFG: applied {} override(s) on top of {}",
        applied,
        mode.label()
    );

    let custom = is_custom_set(&values, mode);
    thresholds::install(values, mode, custom);
    (mode, custom)
}

/// `true` when `values` differ from the bare preset of `mode`.
pub fn is_custom_set(values: &Thresholds, mode: Mode) -> bool {
    *values != mode.preset()
}

/// Parse `data` as one KEY=VALUE per line and apply each known key to
/// `values`.  Returns the number of rows that produced an actual edit.
///
/// Visible for tests in pure host builds.
pub fn apply_overrides(values: &mut Thresholds, data: &[u8]) -> u32 {
    let mut applied = 0u32;

    for (key, value) in crate::cfg_text::kv_lines(data, true) {
        let Ok(key_str) = core::str::from_utf8(key) else {
            continue;
        };
        let Some(value_num) = crate::cfg_text::parse_dec_u32(value) else {
            #[cfg(feature = "embassy-core")]
            defmt::warn!("BORNPETS.CFG: cannot parse value for key");
            continue;
        };

        let mut handled = false;
        for (name, setter) in BORNPETS_CFG_KEYS {
            if key_str.eq_ignore_ascii_case(name) {
                setter(values, value_num);
                applied += 1;
                handled = true;
                break;
            }
        }
        if !handled {
            #[cfg(feature = "embassy-core")]
            defmt::info!("BORNPETS.CFG: unknown key, ignored");
        }
    }

    applied
}

#[cfg(all(test, not(feature = "embassy-core")))]
mod tests {
    use super::*;

    #[test]
    fn applies_known_keys_and_ignores_unknown() {
        let mut t = Thresholds::CLASSIC;
        let n = apply_overrides(
            &mut t,
            b"# comment\nFEED_HUNGER_RELIEF = 4000\nUNKNOWN = 99\nDMG_FULL_LOW_PM=2\n",
        );
        assert_eq!(n, 2);
        assert_eq!(t.FEED_HUNGER_RELIEF, 4000);
        assert_eq!(t.DMG_FULL_LOW_PM, 2);
    }

    #[test]
    fn clamps_u16_overflow() {
        let mut t = Thresholds::CLASSIC;
        apply_overrides(&mut t, b"FEED_HUNGER_RELIEF=999999\n");
        assert_eq!(t.FEED_HUNGER_RELIEF, u16::MAX);
    }

    #[test]
    fn ignores_blank_and_malformed() {
        let mut t = Thresholds::CLASSIC;
        let n = apply_overrides(&mut t, b"\n  \n=\nkey_without_value=\n=value_without_key\n");
        assert_eq!(n, 0);
    }
}
