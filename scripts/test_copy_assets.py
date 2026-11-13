"""Tests for copy_assets.py.  Temp dirs only; never touches a real badge.

Run from the repo root:  python3 -m unittest discover -s scripts -p 'test_copy_assets.py'
"""

import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import copy_assets  # noqa: E402


class CopyAssetsTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.assets = Path(tmp.name) / "assets"
        self.vol = Path(tmp.name) / "vol"
        self.assets.mkdir()
        self.vol.mkdir()
        (self.assets / "000000.PCX").write_bytes(b"asset-000000")
        (self.assets / "010000.PCX").write_bytes(b"asset-010000")
        (self.assets / "PETS.CFG").write_bytes(b"asset-pets")
        (self.assets / "BPETS_V3.CFG").write_bytes(b"# template\n")

    def update(self):
        return copy_assets.update_assets_to(self.vol, self.assets)

    def test_update_leaves_old_config_untouched(self):
        (self.vol / "BORNPETS.CFG").write_bytes(b"MOOD_STEP=40\r\n")
        self.update()
        self.assertEqual((self.vol / "BORNPETS.CFG").read_bytes(), b"MOOD_STEP=40\r\n")
        self.assertEqual(sorted(p.name for p in self.vol.iterdir() if p.suffix.upper() == ".BAK"), [])

    def test_update_adds_v3_config_when_missing(self):
        result = self.update()
        self.assertEqual((self.vol / copy_assets.CONFIG_NAME).read_bytes(), b"# template\n")
        self.assertIn("BPETS_V3.CFG", result["added"])

    def test_update_keeps_edited_v3_config(self):
        (self.vol / "BPETS_V3.CFG").write_bytes(b"MOOD_STEP=40\n")
        result = self.update()
        self.assertEqual((self.vol / "BPETS_V3.CFG").read_bytes(), b"MOOD_STEP=40\n")
        self.assertNotIn("BPETS_V3.CFG", result["added"])

    def test_update_never_overwrites_other_files(self):
        (self.vol / "PETS.CFG").write_bytes(b"mine")
        (self.vol / "000000.PCX").write_bytes(b"badge-specific")
        result = self.update()
        self.assertEqual((self.vol / "PETS.CFG").read_bytes(), b"mine")
        self.assertEqual((self.vol / "000000.PCX").read_bytes(), b"badge-specific")
        self.assertEqual((self.vol / "010000.PCX").read_bytes(), b"asset-010000")
        self.assertEqual(sorted(result["added"]), ["010000.PCX", "BPETS_V3.CFG"])

    def test_update_matches_names_case_insensitively(self):
        (self.vol / "pets.cfg").write_bytes(b"mine")
        self.update()
        self.assertEqual((self.vol / "pets.cfg").read_bytes(), b"mine")
        names = [p.name for p in self.vol.iterdir() if p.name.lower() == "pets.cfg"]
        self.assertEqual(names, ["pets.cfg"])

    def test_update_survives_file_created_after_listing(self):
        real_iterdir = Path.iterdir
        racer = self.assets / "010000.PCX"

        def racing_iterdir(path):
            entries = list(real_iterdir(path))
            if path == self.vol:
                # The badge writes the file after we listed the volume.
                (self.vol / racer.name).write_bytes(b"raced")
            return iter(entries)

        with mock.patch.object(Path, "iterdir", racing_iterdir):
            result = self.update()
        self.assertEqual((self.vol / "010000.PCX").read_bytes(), b"raced")
        self.assertNotIn("010000.PCX", result["added"])

    def test_update_case_variants_in_assets_do_not_overwrite(self):
        (self.assets / "extra.pcx").write_bytes(b"lower")
        (self.assets / "EXTRA.PCX").write_bytes(b"upper")
        result = self.update()
        self.assertEqual(len([n for n in result["added"] if n.lower() == "extra.pcx"]), 1)

    def test_factory_mode_unchanged(self):
        with mock.patch.object(copy_assets, "ASSETS_DIR", self.assets):
            ok = copy_assets.copy_assets_to(self.vol, label="T", quiet=True)
        self.assertTrue(ok)
        for src in self.assets.iterdir():
            self.assertEqual((self.vol / src.name).read_bytes(), src.read_bytes())

    def test_process_device_update_accepts_non_empty_volume(self):
        (self.vol / "000000.PCX").write_bytes(b"badge-specific")
        with mock.patch.object(copy_assets, "ASSETS_DIR", self.assets), \
                mock.patch.object(copy_assets, "mount_via_udisks", return_value=self.vol), \
                mock.patch.object(copy_assets, "unmount_via_udisks"):
            self.assertFalse(copy_assets.process_device("/dev/x", "CYBR0001", quiet=True))
            self.assertTrue(copy_assets.process_device("/dev/x", "CYBR0001", quiet=True, update=True))
        self.assertEqual((self.vol / "000000.PCX").read_bytes(), b"badge-specific")
        self.assertTrue((self.vol / "BPETS_V3.CFG").exists())


if __name__ == "__main__":
    unittest.main()
