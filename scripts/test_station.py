"""Tests for station.py.  No badge, no reader; pyscard is never imported.

Run from the repo root:  python3 -m unittest discover -s scripts -p 'test_station.py'
"""

import contextlib
import io
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import station  # noqa: E402

try:
    import cryptography  # noqa: F401

    HAVE_CRYPTO = True
except ImportError:
    HAVE_CRYPTO = False

VECTORS = Path(__file__).resolve().parent.parent / "tests" / "data" / "station_vectors.txt"
SEED = bytes.fromhex("deadbeeffeedface00112233445566778899aabbccddeeff123456789abcdef0")


class StationTest(unittest.TestCase):
    def test_shared_vectors(self):
        n = 0
        for line in VECTORS.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#"):
                continue
            cols = line.split("\t")
            kind, text = cols[0], cols[1] if len(cols) > 1 else ""
            n += 1
            if kind == "accept":
                fix = station.parse_text(text)
                self.assertIsNotNone(fix, line)
                self.assertEqual(station.canonical(fix), cols[2], line)
            elif kind == "reject":
                with self.assertRaises(station.Reject, msg=line) as cm:
                    station.parse_text(text)
                self.assertEqual(cm.exception.code, cols[2], line)
            elif kind == "ignore":
                self.assertIsNone(station.parse_text(text), line)
            else:
                self.fail("unknown vector kind: " + line)
        self.assertGreater(n, 30)

    def test_presets_parse(self):
        presets = station.load_presets()
        self.assertEqual(len(presets), 8)
        for place, text in presets:
            fix = station.parse_text(text)
            self.assertIsNotNone(fix, place)
            self.assertLessEqual(len(text), station.FIX_MAX_LEN, place)

    def test_fallback_only_for_exact_equivalents(self):
        presets = dict(station.load_presets())
        by_id = {t.split()[1][2:]: t for t in presets.values()}
        self.assertEqual(station.fallback_for(by_id["food1"]), "more food")
        self.assertEqual(station.fallback_for(by_id["pharm1"]), "more drugs")
        self.assertEqual(station.fallback_for(by_id["nap1"]), "sleep like a bear")
        for sid in ("spa1", "bar1", "water1", "gym1", "clinic1"):
            self.assertIsNone(station.fallback_for(by_id[sid]), sid)

    @unittest.skipUnless(HAVE_CRYPTO, "cryptography not installed")
    def test_apdu_layout_and_signature(self):
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

        challenge = bytes(range(16))
        plain = b"fix s=food1 full>60 water>60"
        apdu = station.build_apdu(SEED, challenge, plain)
        self.assertEqual(apdu[:4], bytes([0x80, 0x01, 0x00, 0x00]))
        self.assertEqual(apdu[4], len(plain) + 64)
        self.assertEqual(apdu[5 : 5 + len(plain)], plain)
        self.assertEqual(apdu[-1], 0x00)
        sig = apdu[5 + len(plain) : -1]
        self.assertEqual(len(sig), 64)
        Ed25519PrivateKey.from_private_bytes(SEED).public_key().verify(sig, challenge + plain)

    def test_check_exit_codes(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(station.main(["check", "more food"]), 0)
            self.assertEqual(station.main(["check", "fix s=x full>101"]), 1)


if __name__ == "__main__":
    unittest.main()
