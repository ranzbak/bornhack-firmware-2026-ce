//! Host-only helper for the player's guide:
//!   `guide data --preset classic|casual --out FILE`
//!   `guide words --out FILE`
use bornhack_aegg::game::engine::thresholds::Mode;
use bornhack_aegg::guide::data::dump;
use bornhack_aegg::guide::json::J;

const USAGE: &str =
    "usage: guide data --preset classic|casual --out FILE\n       guide words --out FILE";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (json, out) = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["data", f1, v1, f2, v2] => {
            let (mut preset, mut out) = (None, None);
            for (flag, val) in [(f1, v1), (f2, v2)] {
                match *flag {
                    "--preset" => preset = Some(*val),
                    "--out" => out = Some(*val),
                    _ => usage(),
                }
            }
            let mode = match preset {
                Some("classic") => Mode::Classic,
                Some("casual") => Mode::Casual,
                _ => usage(),
            };
            let Some(out) = out else { usage() };
            (dump(mode), out.to_string())
        }
        ["words", "--out", out] => (bornhack_aegg::guide::words::dump(), out.to_string()),
        _ => usage(),
    };
    write(&json, &out);
}

fn write(json: &J, out: &str) {
    let mut text = json.render();
    text.push('\n');
    if let Err(e) = std::fs::write(out, text) {
        eprintln!("guide: cannot write {out}: {e}");
        std::process::exit(1);
    }
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}
