#!/usr/bin/env python3
"""Unit tests for scripts/build_guide.py check functions."""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_guide as bg  # noqa: E402


class GuideChecks(unittest.TestCase):
    def test_digit_in_prose_fails(self):
        errs = bg.lint_digits("guide/chapters/x.typ", "Feed it 3 times.")
        self.assertEqual(len(errs), 1)
        self.assertIn(":1:", errs[0])
        self.assertIn("typed number in prose", errs[0])

    def test_digits_in_calls_strings_raw_pass(self):
        text = '#v("FULL_LO_PCT") and #w("TYPE 2") and `BPETS_V3.CFG`'
        self.assertEqual(bg.lint_digits("c.typ", text), [])

    def test_code_lines_and_digits_ok_pass(self):
        text = "#let x = 4\n// 12\nA5 paper. // digits-ok: paper size"
        self.assertEqual(bg.lint_digits("c.typ", text), [])

    def test_labels_refs_pass(self):
        self.assertEqual(bg.lint_digits("c.typ", "See @ch-care2 <sec-3>"), [])

    def test_page_count_multiple_of_four(self):
        self.assertEqual(bg.check_pages(8), [])
        self.assertEqual(len(bg.check_pages(10)), 1)

    def test_colophon_must_match_git_describe(self):
        self.assertEqual(len(bg.check_colophon("abc-dirty", "abc")), 1)
        self.assertEqual(bg.check_colophon("abc", "abc"), [])


if __name__ == "__main__":
    unittest.main()
