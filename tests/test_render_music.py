# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: Apache-2.0
import importlib.util
from array import array
from io import BytesIO
import json
import math
from pathlib import Path
import shutil
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
HAS_MIDO = importlib.util.find_spec("mido") is not None


class MusicAssetTests(unittest.TestCase):
    def test_sources_renders_and_instrument_notice_are_present(self):
        for name in (
            "Sabrina_Underwater_Exploration_96BPM",
            "Sabrina_Jellifin_Combat_144BPM",
            "Sabrina_Jellifin_Appears_Stinger",
        ):
            self.assertEqual((ROOT / "art/music" / f"{name}.mid").read_bytes()[:4], b"MThd")
        for name in ("music_calm.ogg", "music_visit.ogg", "jelly_appears.ogg"):
            audio = (ROOT / "assets/audio" / name).read_bytes()
            self.assertEqual(audio[:4], b"OggS")
            self.assertGreater(len(audio), 1000)
        self.assertIn("Frank Wen", (ROOT / "assets/audio/LICENSE-FluidR3.txt").read_text())

    @unittest.skipUnless(shutil.which("ffmpeg"), "FFmpeg is needed to validate decoded audio")
    def test_decoded_audio_has_exact_lengths_headroom_and_smooth_loop_seams(self):
        for name, frames, looping in (
            ("music_calm.ogg", 1764000, True),
            ("music_visit.ogg", 1176001, True),
            ("jelly_appears.ogg", 158025, False),
        ):
            with self.subTest(track=name):
                decoded = subprocess.run(
                    ["ffmpeg", "-v", "error", "-i", str(ROOT / "assets/audio" / name),
                     "-f", "f32le", "-acodec", "pcm_f32le", "-"],
                    check=True, capture_output=True,
                )
                samples = array("f")
                samples.frombytes(decoded.stdout)
                self.assertEqual(len(samples), frames * 2)
                self.assertLess(max(abs(value) for value in samples), 0.95)
                rms = math.sqrt(sum(value * value for value in samples) / len(samples))
                self.assertGreater(rms, 0.001)
                if looping:
                    edge = max(abs(samples[c] - samples[-2 + c]) for c in (0, 1))
                    self.assertLess(edge, 0.03)


@unittest.skipUnless(HAS_MIDO, "Install tools/requirements-music.txt for MIDI authoring tests")
class MusicRenderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("render_music", ROOT / "tools/render_music.py")
        cls.renderer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.renderer)

    def test_real_loops_end_on_bars_not_shortened_note_releases(self):
        for source, expected in (
            ("Sabrina_Underwater_Exploration_96BPM.mid", 40.0),
            ("Sabrina_Jellifin_Combat_144BPM.mid", 26.666688),
        ):
            midi = self.renderer.mido.MidiFile(ROOT / "art/music" / source)
            self.assertGreater(self.renderer.loop_seconds(midi), midi.length)
            self.assertAlmostEqual(self.renderer.loop_seconds(midi), expected, places=6)

    def test_changing_tempo_is_rejected_instead_of_misaligning_the_loop(self):
        midi = self.renderer.mido.MidiFile(
            ROOT / "art/music/Sabrina_Underwater_Exploration_96BPM.mid",
        )
        midi.tracks[0].append(self.renderer.mido.MetaMessage("set_tempo", tempo=500000))
        with self.assertRaisesRegex(ValueError, "constant tempo"):
            self.renderer.loop_seconds(midi)

    def test_warmup_is_discarded_and_output_contains_one_complete_period(self):
        class FakeSynth:
            warmup = 0
            saved = 0
            closed = False

            def __init__(self, _soundfont):
                pass

            def message(self, _message):
                pass

            def samples(self, count, output):
                if count < 0:
                    raise AssertionError("MIDI sample positions must not go backwards")
                if output is None:
                    self.warmup += count
                else:
                    self.saved += count
                    output.write(bytes(count * 8))

            def close(self):
                self.closed = True

        synth = FakeSynth(None)
        with tempfile.TemporaryDirectory() as temporary, patch.object(
            self.renderer, "Synth", return_value=synth,
        ):
            output = Path(temporary) / "loop.f32"
            duration = self.renderer.render_pcm(
                ROOT / "art/music/Sabrina_Jellifin_Combat_144BPM.mid",
                Path("soundfont.sf2"), output, True,
            )
            expected = round(duration * self.renderer.SAMPLE_RATE)
            self.assertEqual(synth.warmup, expected)
            self.assertEqual(synth.saved, expected + round(0.03 * self.renderer.SAMPLE_RATE))
            self.assertEqual(output.stat().st_size, expected * 8)
            self.assertTrue(synth.closed)

    def test_seam_blend_preserves_length_and_joins_the_next_pass(self):
        first = array("f", [0.1, 0.2] * 10).tobytes()
        continuation = array("f", [0.3, 0.4] * 4).tobytes()
        output = BytesIO(first)
        self.renderer.blend_loop_start(output, continuation)
        result = array("f")
        result.frombytes(output.getvalue())
        self.assertEqual(len(output.getvalue()), len(first))
        self.assertAlmostEqual(result[0], 0.3)
        self.assertAlmostEqual(result[1], 0.4)
        self.assertAlmostEqual(result[6], 0.1)
        self.assertAlmostEqual(result[7], 0.2)

    def test_stinger_has_a_release_tail_and_no_warmup(self):
        midi = self.renderer.mido.MidiFile(
            ROOT / "art/music/Sabrina_Jellifin_Appears_Stinger.mid",
        )
        expected = round((midi.length + 2.0) * self.renderer.SAMPLE_RATE)
        synth = SimpleNamespace(
            samples=lambda count, output: output.write(bytes(count * 8)),
            message=lambda message: None,
            close=lambda: None,
        )
        with tempfile.TemporaryDirectory() as temporary, patch.object(
            self.renderer, "Synth", return_value=synth,
        ):
            output = Path(temporary) / "stinger.f32"
            self.renderer.render_pcm(
                ROOT / "art/music/Sabrina_Jellifin_Appears_Stinger.mid",
                Path("soundfont.sf2"), output, False,
            )
            self.assertEqual(output.stat().st_size, expected * 8)

    def test_encoding_uses_static_gain_and_accepts_ffmpeg_trailing_statistics(self):
        levels = {"input_i": "-24.0", "input_tp": "-8.0"}
        result = SimpleNamespace(stderr=json.dumps(levels) + "\n[out] encoding statistics\n")
        with patch.object(self.renderer.subprocess, "run", return_value=result) as run:
            self.renderer.encode(Path("loop.f32"), Path("loop.ogg"), 40.0, True)
            command = run.call_args_list[1].args[0]
            self.assertEqual(command[command.index("-af") + 1], "volume=4.0000dB")
            self.assertNotIn("afade", " ".join(command))
            self.renderer.encode(Path("stinger.f32"), Path("stinger.ogg"), 3.5, False)
            self.assertIn("afade=t=out", " ".join(run.call_args.args[0]))


if __name__ == "__main__":
    unittest.main()
