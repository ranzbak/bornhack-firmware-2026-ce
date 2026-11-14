#!/usr/bin/env python3
"""Build the BornPets player's guide PDF with Typst.

  build_guide.py                 clean guide/build/, all inputs, compile, check
  build_guide.py --stage NAME    rebuild only those inputs (repeatable), compile, check
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
STAGES = ()  # Tasks 2 to 4 add: data, words, screens, sprites
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


def build_inputs(stages):
    """Tasks 2 to 4 add the stages data, words, screens and sprites."""
    for s in stages:
        if s not in STAGES:
            sys.exit(f"guide: unknown stage {s!r}")


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


def check(commit):
    errors = []
    for p in sorted(glob.glob("guide/chapters/*.typ")):
        errors += lint_digits(p, open(p).read())
    errors += check_pages(query("<guide-end>", commit))
    errors += check_colophon(query("<colophon-commit>", commit), commit)
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
    compile_pdf(commit)
    check(commit)


if __name__ == "__main__":
    main()
