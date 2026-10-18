# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
"""Exercise real rustfmt with Windows-style default text encodings."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools import export_rust_contacts as exporter


class ContactExportTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.target = self.root / "src/reef_sim/src/generated_contacts.rs"
        for relative in (
            "art/characters/sabrina.contacts.json",
            "src/reef_sim/src/generated_contacts.rs",
        ):
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(exporter.ROOT / relative, destination)
        self.enterContext(patch.object(exporter, "ROOT", self.root))

        run = subprocess.run
        read = Path.read_text
        write = Path.write_text

        def windows_run(*args, **kwargs):
            kwargs.setdefault("encoding", "cp1252")
            return run(*args, **kwargs)

        def windows_read(path, *args, **kwargs):
            kwargs.setdefault("encoding", "cp1252")
            return read(path, *args, **kwargs)

        def windows_write(path, *args, **kwargs):
            kwargs.setdefault("encoding", "cp1252")
            return write(path, *args, **kwargs)

        self.enterContext(patch.object(subprocess, "run", side_effect=windows_run))
        self.enterContext(patch.object(Path, "read_text", windows_read))
        self.enterContext(patch.object(Path, "write_text", windows_write))

    def test_check_handles_utf8_source_with_windows_defaults(self):
        before = self.target.read_bytes()
        exporter.export(check=True)
        self.assertEqual(self.target.read_bytes(), before)

    def test_generated_source_round_trips_as_utf8_with_lf(self):
        self.target.unlink()
        exporter.export()
        data = self.target.read_bytes()
        self.assertIn("Luka Dežulović", data.decode("utf-8"))
        self.assertNotIn(b"\r\n", data)
        exporter.export(check=True)
        self.target.write_bytes(data.replace(b"\n", b"\r\n"))
        exporter.export(check=True)

    def test_check_still_rejects_stale_generated_source(self):
        self.target.write_bytes(b"// stale\n")
        with self.assertRaisesRegex(AssertionError, "export_rust_contacts"):
            exporter.export(check=True)


if __name__ == "__main__":
    unittest.main()
