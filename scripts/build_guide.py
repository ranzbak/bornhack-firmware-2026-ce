#!/usr/bin/env python3
"""Build the BornPets player's guide PDF with Typst.

  build_guide.py                 clean guide/build/, all inputs, compile, check
  build_guide.py --stage NAME    rebuild only those inputs (repeatable), compile, check
                                 (stages: data, words; each rewrites only its own files)
  build_guide.py --pdf-only      no inputs, only compile and check
"""
import argparse
import glob
import os
import re
import shutil
import subprocess
import sys

ROOT = ""
OUT_PDF = "guide/out/bornpets-guide.pdf"
STAGES = ("data", "words")  # Task 4 adds: screens, sprites
GUIDE_BIN = "target/x86_64-unknown-linux-gnu/debug/guide"
PRESETS = ("classic", "casual")
DATA_DIR = "guide/build/data"
WORDS_JSON = f"{DATA_DIR}/words.json"
COVERED_GROUPS = "guide/covered-groups"
# Inputs `--pdf-only` needs, and the stage that makes each.
INPUTS = ([(f"{DATA_DIR}/{p}.json", "data") for p in PRESETS] + [(WORDS_JSON, "words")])
_built = False
SKIP_PREFIXES = ("//", "#let", "#import", "#set", "#show", "#include")
DIGITS_OK = re.compile(r"//\s*digits-ok:\s*\S")


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def typst_args(commit):
    return ["--root", ".", "--font-path", "guide/fonts", "--ignore-system-fonts",
            "--input", f"commit={commit}"]


def strip_balanced_calls(line):
    """Remove `#name(...)` calls, parentheses balanced on this line."""
    out, i = [], 0
    while i < len(line):
        m = re.match(r"#[A-Za-z_][\w.-]*\(", line[i:])
        if not m:
            out.append(line[i])
            i += 1
            continue
        depth, j = 1, i + m.end()
        while j < len(line) and depth:
            depth += {"(": 1, ")": -1}.get(line[j], 0)
            j += 1
        i = j
    return "".join(out)


def lint_digits(path, text):
    errors = []
    for n, line in enumerate(text.splitlines(), 1):
        s = line.strip()
        if s.startswith(SKIP_PREFIXES) or DIGITS_OK.search(line):
            continue
        s = re.sub(r'"(?:\\.|[^"\\])*"', "", line)
        s = re.sub(r"`[^`]*`", "", s)
        s = re.sub(r"<[\w:.-]+>", "", s)
        s = re.sub(r"@[\w:.-]+", "", s)
        s = strip_balanced_calls(s)
        if re.search(r"\d", s):
            errors.append(f"{path}:{n}: typed number in prose")
    return errors


def check_pages(end_page):
    if end_page % 4 != 0:
        return [f"page count {end_page} is not a multiple of 4"]
    return []


def check_colophon(found, expected):
    if found != expected:
        return [f"colophon commit {found!r} does not match build commit {expected!r}"]
    return []


def check_typst():
    want = open("guide/typst-version").read().strip()
    try:
        out = subprocess.check_output(["typst", "--version"], text=True).strip()
    except FileNotFoundError:
        out = "nothing"
    found = out.split()[1] if out != "nothing" else out
    if found != want:
        sys.exit(f"guide: need typst {want}, found {found}. Install: "
                 f"CARGO_BUILD_JOBS=2 taskset -c 2,3 cargo install typst-cli --locked --version {want} -j 2")


def build_guide_bin():
    """Build the host-only `guide` bin once per run."""
    global _built
    if _built:
        return
    jobs = os.environ.get("CARGO_BUILD_JOBS", "2")
    subprocess.run(["cargo", "build", "--features", "simulator", "--bin", "guide",
                    "-j", jobs], check=True)
    if not os.path.exists(GUIDE_BIN):
        sys.exit(f"guide: cargo built the bin but {GUIDE_BIN} is missing; "
                 "check the target directory in .cargo/config.toml")
    _built = True


def remove(path):
    if os.path.exists(path):
        os.remove(path)


def stage_data():
    """Rewrites only the preset files: the words stage owns words.json."""
    build_guide_bin()
    os.makedirs(DATA_DIR, exist_ok=True)
    for preset in PRESETS:
        out = f"{DATA_DIR}/{preset}.json"
        remove(out)
        subprocess.run([GUIDE_BIN, "data", "--preset", preset, "--out", out], check=True)


def stage_words():
    """Rewrites only words.json: the data stage owns the preset files."""
    build_guide_bin()
    os.makedirs(DATA_DIR, exist_ok=True)
    remove(WORDS_JSON)
    subprocess.run([GUIDE_BIN, "words", "--out", WORDS_JSON], check=True)


def require_inputs():
    """Exit 1, naming the stage, when a built input is missing."""
    for path, stage in INPUTS:
        if not os.path.exists(path):
            sys.exit(f"guide: {path} is missing; run build_guide.py --stage {stage}")


def build_inputs(stages):
    """Task 4 adds the stages screens and sprites."""
    for s in stages:
        if s not in STAGES:
            sys.exit(f"guide: unknown stage {s!r}")
    if "data" in stages:
        stage_data()
    if "words" in stages:
        stage_words()


def compile_pdf(commit):
    os.makedirs("guide/out", exist_ok=True)
    env = dict(os.environ, SOURCE_DATE_EPOCH=git("log", "-1", "--format=%ct"))
    subprocess.run(["typst", "compile", *typst_args(commit), "guide/main.typ", OUT_PDF],
                   check=True, env=env)


def query(label, commit):
    env = dict(os.environ, SOURCE_DATE_EPOCH=git("log", "-1", "--format=%ct"))
    out = subprocess.check_output(
        ["typst", "query", *typst_args(commit), "guide/main.typ", label,
         "--field", "value", "--one"], text=True, env=env)
    import json
    return json.loads(out)


def check_marks(marked, words, groups):
    """`marked`: the texts the guide wrote with `w()`. `words`: words.json's
    entries. `groups`: the groups the guide must explain in full."""
    texts = {w["text"] for w in words}
    errors = [f"marked word not on the badge: {t}" for t in sorted(marked - texts)]
    errors += [f"unexplained {w['group']} word: {w['text']}"
               for w in words if w["group"] in groups and w["text"] not in marked]
    return errors


def covered_groups(text, words):
    """The lines of guide/covered-groups (`text`), or every group when there
    is no such file (`text` is None)."""
    if text is None:
        return {w["group"] for w in words}
    return {line.strip() for line in text.splitlines() if line.strip()}


def marked_words(commit):
    env = dict(os.environ, SOURCE_DATE_EPOCH=git("log", "-1", "--format=%ct"))
    out = subprocess.check_output(
        ["typst", "query", *typst_args(commit), "guide/main.typ", "<gw>",
         "--field", "value"], text=True, env=env)
    import json
    return set(json.loads(out))


def check_words(commit):
    import json
    words = json.load(open(WORDS_JSON))["words"]
    text = open(COVERED_GROUPS).read() if os.path.exists(COVERED_GROUPS) else None
    return check_marks(marked_words(commit), words, covered_groups(text, words))


def check(commit):
    errors = []
    for p in sorted(glob.glob("guide/chapters/*.typ")):
        errors += lint_digits(p, open(p).read())
    errors += check_pages(query("<guide-end>", commit))
    errors += check_colophon(query("<colophon-commit>", commit), commit)
    errors += check_words(commit)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print(f"guide: ok, {query('<guide-end>', commit)} pages, build {commit}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--stage", action="append", default=[])
    ap.add_argument("--pdf-only", action="store_true")
    a = ap.parse_args()
    os.chdir(git("rev-parse", "--show-toplevel"))
    check_typst()
    commit = git("describe", "--always", "--dirty=-dirty", "--abbrev=12")
    if not a.pdf_only:
        if not a.stage:
            shutil.rmtree("guide/build", ignore_errors=True)
        os.makedirs("guide/build", exist_ok=True)
        build_inputs(a.stage or list(STAGES))
    require_inputs()
    compile_pdf(commit)
    check(commit)


if __name__ == "__main__":
    main()
