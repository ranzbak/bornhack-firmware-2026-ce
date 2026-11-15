#!/usr/bin/env python3
"""Station reader tool for organisers: check, sign and send BornPets station messages.

A Python 3 stdlib mirror of src/game/engine/fix.rs (the shared vectors in
tests/data/station_vectors.txt keep the two in step), plus a signer and a PC/SC sender.
`cryptography` is imported only to sign, `pyscard` only to send.

    station.py check TEXT...
    station.py presets
    station.py apdu --key F --challenge HEX TEXT
    station.py send --key F (--preset PLACE | --text TEXT) [--reader N] [--once]
"""

import argparse
import re
import sys
import time
from pathlib import Path

FIX_MAX_LEN = 96
STATION_ID_MAX = 8
PRESETS = str(Path(__file__).resolve().parent / "station_presets.txt")

ASCII_WS = " \t\n\f\r"  # Rust's u8::is_ascii_whitespace
OPS = "=><+-"
TARGETS = ("full", "water", "energy", "fun", "dirt", "booze", "strain", "fit", "weight", "health")
CONDITIONS = ("hangover", "injury", "groggy", "deprived", "dependence", "diabetes")

# The old phrases, as station messages (the same four as fix::ALIASES).
ALIASES = {
    "more food": "fix s=food full>60 water>60",
    "more drugs": "fix s=drugs health>100 booze<20 clear=hangover,injury,groggy",
    "sleep like a bear": "fix s=rest energy>100 clear=groggy,deprived",
    "more inspiration": "fix s=inspire fun>60",
}

SW_APPLIED = 0x9000
SW_REFUSED = 0x6985
SW_UNKNOWN = 0x6A82


class Reject(Exception):
    """A text that starts like a station message but is refused. `code` is Reject::code."""

    def __init__(self, code):
        super().__init__(code)
        self.code = code


def normalize(text):
    """Trim ASCII whitespace, lowercase, collapse whitespace runs to one space.

    None when the result is empty, holds a non-ASCII character, or exceeds FIX_MAX_LEN."""
    if not text.isascii():
        return None
    out = " ".join(w for w in re.split("[" + re.escape(ASCII_WS) + "]+", text) if w).lower()
    if not out or len(out) > FIX_MAX_LEN:
        return None
    return out


def _valid_id(s):
    return 1 <= len(s) <= STATION_ID_MAX and re.fullmatch("[a-z0-9]+", s) is not None


def _digits(s, max_digits):
    if not s or len(s) > max_digits or re.fullmatch("[0-9]+", s) is None:
        return None
    return int(s)


def _parse(normalized):
    """None when the first word is not "fix"; else the fix dict or Reject."""
    tokens = normalized.split(" ")
    if tokens[0] != "fix":
        return None
    if len(tokens) < 2 or not tokens[1].startswith("s=") or not _valid_id(tokens[1][2:]):
        raise Reject("bad_id")
    fix = {"id": tokens[1][2:], "levels": [], "clear": [], "cd": None, "skipped": []}
    cd_seen = False
    for tok in tokens[2:]:
        at = next((i for i, c in enumerate(tok) if c in OPS), None)
        if at is None:
            raise Reject("bad_item")
        name, op, rest = tok[:at], tok[at], tok[at + 1 :]
        if not name or not rest or re.fullmatch("[a-z]+", name) is None:
            raise Reject("bad_item")
        if name in TARGETS:
            v = _digits(rest, 3)
            if v is None:
                raise Reject("bad_item")
            if v > 100:
                raise Reject("over_100")
            fix["levels"].append((name, op, v))
        elif name == "clear":
            if op != "=":
                raise Reject("bad_item")
            for word in rest.split(","):
                if not word or re.fullmatch("[a-z]+", word) is None:
                    raise Reject("bad_item")
                if word in CONDITIONS:
                    if word not in fix["clear"]:
                        fix["clear"].append(word)
                else:
                    fix["skipped"].append(word)
        elif name == "cd":
            if op != "=":
                raise Reject("bad_item")
            n = _digits(rest, 4)
            if n is None:
                raise Reject("bad_item")
            if cd_seen:
                raise Reject("duplicate")
            cd_seen = True
            fix["cd"] = n
        elif name == "s":
            raise Reject("duplicate")
        else:
            fix["skipped"].append(name)
    if not fix["levels"] and not fix["clear"]:
        raise Reject("nothing_known")
    fix["clear"].sort(key=CONDITIONS.index)
    return fix


def parse_text(text):
    """normalize, expand alias, parse.

    Returns {"id", "levels": [(name, op, value)], "clear": [names], "cd": int|None,
    "skipped": [unknown names]}, or None when it is not a station message. Raises Reject."""
    n = normalize(text)
    if n is None:
        start = text.lstrip(ASCII_WS)
        if start[:3].lower() == "fix" and (len(start) == 3 or start[3] in ASCII_WS):
            raise Reject("unreadable")
        return None
    return _parse(ALIASES.get(n, n))


def canonical(fix):
    """The normal form the shared vectors print (Fix::canonical)."""
    parts = ["fix s=" + fix["id"]]
    parts += [f"{name}{op}{v}" for name, op, v in fix["levels"]]
    if fix["clear"]:
        parts.append("clear=" + ",".join(fix["clear"]))
    if fix["cd"] is not None:
        parts.append(f"cd={fix['cd']}")
    return " ".join(parts)


def load_presets(path=PRESETS):
    """(place, plaintext) per line of the presets file."""
    out = []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if line.strip() and not line.startswith("#"):
            place, text = line.split("|", 1)
            out.append((place, text))
    return out


def _items(text):
    """The canonical items of a text, without the station id."""
    return canonical(parse_text(text)).split(" ", 2)[2:]


def fallback_for(plaintext):
    """The old phrase whose fix has the same items (ignoring s=), else None."""
    want = _items(plaintext)
    for phrase in ALIASES:
        if _items(phrase) == want:
            return phrase
    return None


def build_apdu(seed, challenge, plaintext):
    """80 01 00 00 Lc plaintext||signature 00; the signature covers challenge||plaintext."""
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

    sig = Ed25519PrivateKey.from_private_bytes(seed).sign(challenge + plaintext)
    payload = plaintext + sig
    if len(payload) > 255:
        raise ValueError("payload too long for a short-form APDU")
    return bytes([0x80, 0x01, 0x00, 0x00, len(payload)]) + payload + b"\x00"


def send(conn, seed, plaintext):
    """GET CHALLENGE, then SIGNED CMD, on a pyscard-style connection. Returns the SW."""
    data, sw1, sw2 = conn.transmit([0x80, 0x02, 0x00, 0x00, 0x00])
    if (sw1, sw2) != (0x90, 0x00) or len(data) != 16:
        raise RuntimeError(f"GET CHALLENGE failed: SW={sw1:02X}{sw2:02X}")
    _, sw1, sw2 = conn.transmit(list(build_apdu(seed, bytes(data), plaintext)))
    return (sw1 << 8) | sw2


def _read_key(path):
    raw = Path(path).read_bytes()
    if len(raw) != 32:
        try:
            raw = bytes.fromhex(raw.decode("ascii").strip())
        except ValueError:
            raw = b""
    if len(raw) != 32:
        sys.exit(f"{path}: expected a 32-byte seed (raw or 64 hex digits)")
    return raw


def _describe(code):
    return {
        SW_APPLIED: "applied",
        SW_REFUSED: "refused (cooldown, gone or no pet)",
        SW_UNKNOWN: "not understood",
    }.get(code, f"unexpected status {code:04X}")


def _send_with_fallback(conn, seed, plaintext):
    """Send; on 6A 82 (never on 69 85) retry with the old phrase when one is equivalent."""
    sw = send(conn, seed, plaintext)
    if sw == SW_UNKNOWN:
        alias = fallback_for(plaintext)
        if alias:
            print(f'old firmware? sending the old phrase "{alias}"')
            sw = send(conn, seed, alias.encode("ascii"))
    return sw


def _connect(reader):
    try:
        conn = reader.createConnection()
        conn.connect()
        return conn
    except Exception:  # no card yet, or an unreadable one
        return None


def _wait_gone(reader):
    while True:
        conn = _connect(reader)
        if conn is None:
            return
        try:
            conn.transmit([0x00, 0xA4, 0x04, 0x00, 0x00])  # any APDU; fails once the badge left
            conn.disconnect()
        except Exception:
            return
        time.sleep(0.3)


def _cmd_send(args):
    seed = _read_key(args.key)
    if args.preset:
        found = [t for p, t in load_presets() if p.lower() == args.preset.lower()]
        if not found:
            sys.exit("no such preset; see: station.py presets")
        text = found[0]
    else:
        text = args.text
    try:
        fix = parse_text(text)
    except Reject as e:
        sys.exit(f"rejected: {e.code}")
    if fix is None:
        sys.exit("not a station message")
    plaintext = normalize(text).encode("ascii")

    from smartcard.System import readers  # pyscard, only needed to send

    names = readers()
    if not names:
        sys.exit("no PC/SC reader found")
    reader = names[args.reader]
    print(f"{canonical(fix)}  (reader: {reader})")
    while True:
        print("waiting for a badge...")
        conn = None
        while conn is None:
            conn = _connect(reader)
            if conn is None:
                time.sleep(0.3)
        try:
            print(_describe(_send_with_fallback(conn, seed, plaintext)))
        except Exception as e:
            print(f"error: {e}")
        if args.once:
            return 0
        _wait_gone(reader)


def main(argv=None):
    ap = argparse.ArgumentParser(description="BornPets station reader tool for organisers")
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("check", help="canonical form and skipped items of each text")
    p.add_argument("texts", nargs="+", metavar="TEXT")
    sub.add_parser("presets", help="the preset table with each fallback alias")
    p = sub.add_parser("apdu", help="the SIGNED CMD bytes in hex")
    p.add_argument("--key", required=True)
    p.add_argument("--challenge", required=True, help="32 hex digits")
    p.add_argument("text", metavar="TEXT")
    p = sub.add_parser("send", help="wait for a badge, send, report")
    p.add_argument("--key", required=True)
    g = p.add_mutually_exclusive_group(required=True)
    g.add_argument("--preset", metavar="PLACE")
    g.add_argument("--text")
    p.add_argument("--reader", type=int, default=0)
    p.add_argument("--once", action="store_true")
    args = ap.parse_args(argv)

    if args.cmd == "check":
        bad = 0
        for text in args.texts:
            try:
                fix = parse_text(text)
            except Reject as e:
                print(f"{text!r}: rejected ({e.code})")
                bad += 1
                continue
            if fix is None:
                print(f"{text!r}: not a station message")
                bad += 1
                continue
            skipped = f"  skipped: {', '.join(fix['skipped'])}" if fix["skipped"] else ""
            print(f"{text!r}: {canonical(fix)}{skipped}")
        return 1 if bad else 0
    if args.cmd == "presets":
        for place, text in load_presets():
            print(f"{place:<14} {text}")
            alias = fallback_for(text)
            if alias:
                print(f"{'':<14} old firmware: {alias}")
        return 0
    if args.cmd == "apdu":
        try:
            fix = parse_text(args.text)
        except Reject as e:
            sys.exit(f"rejected: {e.code}")
        if fix is None:
            sys.exit("not a station message")
        plaintext = normalize(args.text).encode("ascii")
        print(build_apdu(_read_key(args.key), bytes.fromhex(args.challenge), plaintext).hex())
        return 0
    return _cmd_send(args)


if __name__ == "__main__":
    sys.exit(main())
