//! Host-only helper for the player's guide: `guide data --preset classic|casual --out FILE`.
use bornhack_aegg::game::engine::thresholds::Mode;
use bornhack_aegg::guide::data::dump;

const USAGE: &str = "usage: guide data --preset classic|casual --out FILE";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cmd, f1, v1, f2, v2] = args.as_slice() else {
        usage()
    };
    if cmd != "data" {
        usage()
    }
    let (mut preset, mut out) = (None, None);
    for (flag, val) in [(f1, v1), (f2, v2)] {
        match flag.as_str() {
            "--preset" => preset = Some(val.clone()),
            "--out" => out = Some(val.clone()),
            _ => usage(),
        }
    }
    let mode = match preset.as_deref() {
        Some("classic") => Mode::Classic,
        Some("casual") => Mode::Casual,
        _ => usage(),
    };
    let Some(out) = out else { usage() };
    let mut text = dump(mode).render();
    text.push('\n');
    if let Err(e) = std::fs::write(&out, text) {
        eprintln!("guide: cannot write {out}: {e}");
        std::process::exit(1);
    }
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}
