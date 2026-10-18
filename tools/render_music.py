#!/usr/bin/env python3
# Copyright (c) 2026 Luka Dežulović
# SPDX-License-Identifier: MIT
"""Render the supplied MIDI arrangements with FluidSynth, then encode runtime Oggs."""

import argparse
from array import array
import ctypes
from ctypes.util import find_library
import json
from io import BytesIO
import math
from pathlib import Path
import subprocess
import tempfile

import mido

ROOT = Path(__file__).resolve().parents[1]
SAMPLE_RATE = 44100
TRACKS = (
    ("Sabrina_Underwater_Exploration_96BPM.mid", "music_calm.ogg", True),
    ("Sabrina_Jellifin_Combat_144BPM.mid", "music_visit.ogg", True),
    ("Sabrina_Jellifin_Appears_Stinger.mid", "jelly_appears.ogg", False),
)


def loop_seconds(midi: mido.MidiFile) -> float:
    """Pad the final note release to the bar boundary, not the last MIDI event."""
    tempos = {msg.tempo for track in midi.tracks for msg in track if msg.type == "set_tempo"}
    signatures = {
        (msg.numerator, msg.denominator)
        for track in midi.tracks for msg in track if msg.type == "time_signature"
    }
    if len(tempos) != 1 or len(signatures) != 1:
        raise ValueError("Music loops must have one constant tempo and time signature")
    numerator, denominator = signatures.pop()
    ticks_per_bar = midi.ticks_per_beat * numerator * 4 / denominator
    ticks = max(sum(msg.time for msg in track) for track in midi.tracks)
    bars = math.ceil(ticks / ticks_per_bar)
    if bars < 1:
        raise ValueError("Music loop is empty")
    return mido.tick2second(bars * ticks_per_bar, midi.ticks_per_beat, tempos.pop())


class Synth:
    def __init__(self, soundfont: Path):
        library = find_library("fluidsynth")
        if library is None:
            raise RuntimeError("Install the FluidSynth shared library before rendering music")
        self.library = ctypes.CDLL(library)
        self.settings = None
        self.synth = None
        pointer = ctypes.c_void_p
        integer = ctypes.c_int
        self.new_settings = self.bind("new_fluid_settings", pointer, [])
        self.delete_settings = self.bind("delete_fluid_settings", None, [pointer])
        self.set_number = self.bind(
            "fluid_settings_setnum", integer, [pointer, ctypes.c_char_p, ctypes.c_double],
        )
        self.new_synth = self.bind("new_fluid_synth", pointer, [pointer])
        self.delete_synth = self.bind("delete_fluid_synth", None, [pointer])
        self.load = self.bind("fluid_synth_sfload", integer, [pointer, ctypes.c_char_p, integer])
        self.write = self.bind(
            "fluid_synth_write_float", integer,
            [pointer, integer, pointer, integer, integer, pointer, integer, integer],
        )
        self.handlers = {
            "note_on": (self.bind("fluid_synth_noteon", integer, [pointer] + [integer] * 3),
                        ("channel", "note", "velocity")),
            "note_off": (self.bind("fluid_synth_noteoff", integer, [pointer] + [integer] * 2),
                         ("channel", "note")),
            "program_change": (self.bind("fluid_synth_program_change", integer, [pointer] + [integer] * 2),
                               ("channel", "program")),
            "control_change": (self.bind("fluid_synth_cc", integer, [pointer] + [integer] * 3),
                               ("channel", "control", "value")),
        }
        try:
            self.settings = self.new_settings()
            if not self.settings:
                raise RuntimeError("Cannot allocate FluidSynth settings")
            self.check(self.set_number(self.settings, b"synth.sample-rate", SAMPLE_RATE))
            self.synth = self.new_synth(self.settings)
            if not self.synth or self.load(self.synth, str(soundfont).encode(), 1) < 0:
                raise RuntimeError(f"Cannot load soundfont: {soundfont}")
        except Exception:
            self.close()
            raise

    def bind(self, name: str, result, arguments):
        function = getattr(self.library, name)
        function.restype = result
        function.argtypes = arguments
        return function

    @staticmethod
    def check(result: int) -> None:
        if result != 0:
            raise RuntimeError(f"FluidSynth failed with status {result}")

    def message(self, message: mido.Message) -> None:
        if message.is_meta:
            return
        if message.type not in self.handlers:
            raise ValueError(f"Unsupported MIDI message: {message}")
        handler, fields = self.handlers[message.type]
        self.check(handler(self.synth, *(getattr(message, field) for field in fields)))

    def samples(self, count: int, output) -> None:
        while count > 0:
            frames = min(count, 4096)
            buffer = (ctypes.c_float * (frames * 2))()
            self.check(self.write(self.synth, frames, buffer, 0, 2, buffer, 1, 2))
            if output is not None:
                output.write(bytes(buffer))
            count -= frames

    def close(self) -> None:
        if self.synth:
            self.delete_synth(self.synth)
            self.synth = None
        if self.settings:
            self.delete_settings(self.settings)
            self.settings = None


def render_pcm(source: Path, soundfont: Path, destination: Path, looping: bool) -> float:
    midi = mido.MidiFile(source)
    duration = loop_seconds(midi) if looping else midi.length + 2.0
    frames = round(duration * SAMPLE_RATE)
    synth = Synth(soundfont)
    try:
        with destination.open("w+b") as output:
            # A discarded first pass carries notes and reverb across the loop seam.
            for repeat in range(2 if looping else 1):
                target = None if looping and repeat == 0 else output
                render_pass(midi, synth, frames, target)
            if looping:
                continuation = BytesIO()
                render_pass(midi, synth, round(0.03 * SAMPLE_RATE), continuation)
                blend_loop_start(output, continuation.getvalue())
    finally:
        synth.close()
    return frames / SAMPLE_RATE


def render_pass(midi: mido.MidiFile, synth: Synth, frames: int, output) -> None:
    cursor = 0
    seconds = 0.0
    for message in midi:
        seconds += message.time
        position = round(seconds * SAMPLE_RATE)
        if position > frames:
            break
        synth.samples(position - cursor, output)
        synth.message(message)
        cursor = position
    synth.samples(frames - cursor, output)


def blend_loop_start(output, continuation: bytes) -> None:
    """Bridge free-running synth/reverb phases without shortening the musical bar."""
    following = array("f")
    following.frombytes(continuation)
    output.seek(0)
    beginning = array("f")
    beginning.frombytes(output.read(len(continuation)))
    frames = len(following) // 2
    for index in range(len(beginning)):
        mix = (1.0 - math.cos(math.pi * (index // 2) / (frames - 1))) * 0.5
        beginning[index] = following[index] * (1.0 - mix) + beginning[index] * mix
    output.seek(0)
    output.write(beginning.tobytes())


def encode(pcm: Path, destination: Path, duration: float, looping: bool) -> None:
    command = [
        "ffmpeg", "-hide_banner", "-nostdin", "-y",
        "-f", "f32le", "-ar", str(SAMPLE_RATE), "-ac", "2", "-i", str(pcm),
    ]
    measured = subprocess.run(
        command + ["-af", "loudnorm=I=-20:TP=-2:LRA=11:print_format=json", "-f", "null", "-"],
        check=True, capture_output=True, text=True,
    )
    levels, _ = json.JSONDecoder().raw_decode(measured.stderr[measured.stderr.rfind("{"):])
    loudness, peak = float(levels["input_i"]), float(levels["input_tp"])
    if not math.isfinite(loudness) or not math.isfinite(peak):
        raise ValueError(f"Rendered audio is silent or invalid: {pcm}")
    gain = min(-20.0 - loudness, -2.0 - peak)
    # Static gain preserves loop continuity, unlike a time-varying normalizer.
    filters = f"volume={gain:.4f}dB"
    if not looping:
        filters += f",afade=t=out:st={duration - 0.05:.6f}:d=0.05"
    subprocess.run(
        command + ["-af", filters, "-c:a", "libvorbis", "-q:a", "6", str(destination)],
        check=True, capture_output=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--soundfont", type=Path, required=True)
    args = parser.parse_args()
    if not args.soundfont.is_file():
        parser.error(f"Soundfont does not exist: {args.soundfont}")
    try:
        with tempfile.TemporaryDirectory(prefix="sabrina-music-", dir=ROOT / "assets/audio") as temporary:
            for source, name, looping in TRACKS:
                pcm = Path(temporary) / f"{name}.f32"
                duration = render_pcm(ROOT / "art/music" / source, args.soundfont, pcm, looping)
                staged = Path(temporary) / name
                encode(pcm, staged, duration, looping)
                staged.replace(ROOT / "assets/audio" / name)
                print(f"{name}: {duration:.6f} seconds, {'loop' if looping else 'one-shot'}")
    except subprocess.CalledProcessError as error:
        parser.exit(1, f"FFmpeg failed:\n{error.stderr}\n")


if __name__ == "__main__":
    main()
