#!/bin/env python

import hashlib
import re
import subprocess
import sys
from pathlib import Path
from typing import List

import cv2

from av1an.ffmpeg import frame_probe_ffmpeg
from av1an.project import Project
from av1an.vapoursynth import frame_probe_vspipe, is_vapoursynth


def terminate():
    sys.exit(1)


def get_project_priority(project: Project):
    # Process priority only supported on windows.
    if sys.platform != 'win32':
        return 0

    priorities = {
        'idle': subprocess.IDLE_PRIORITY_CLASS,
        'low': subprocess.BELOW_NORMAL_PRIORITY_CLASS,
        'normal': subprocess.NORMAL_PRIORITY_CLASS,
        'high': subprocess.HIGH_PRIORITY_CLASS,
    }
    return priorities[project.priority]


def hash_path(s: str) -> int:
    """
    Return hash of full path to file
    :param s: string
    """
    assert isinstance(s, str)

    return str(hashlib.sha3_512(s.encode()).hexdigest())[-8:]


def get_cq(command):
    """
    Return cq values from command
    :param command: string with commands for encoder
    :return: list with frame numbers of keyframes

    """
    matches = re.findall(r"--cq-level= *([^ ]+?) ", command)
    return int(matches[-1])


def list_index_of_regex(lst: List[str], regex_str: str) -> int:
    """
    Gets the first index of the list where regex_str matches

    :param lst: the list
    :param regex_str: the regex as a string
    :return: the index where regex_str appears in the list
    :raises ValueError: if regex_str is not found
    """
    reg = re.compile(regex_str)
    for i, elem in enumerate(lst):
        if reg.match(elem):
            return i
    raise ValueError(f"{reg} is not in list")


def frame_probe_fast(source: Path, is_vs: bool = False):
    """
    Consolidated function to retrieve the number of frames from the input quickly,
    falls back on a slower (but accurate) frame count if a quick count cannot be found.

    Handles vapoursynth input as well.
    """
    total = 0
    if not is_vs:
        try:
            import vapoursynth
            from vapoursynth import core

            plugins = vapoursynth.get_core().get_plugins()
            if "systems.innocent.lsmas" in plugins:
                total = core.lsmas.LWLibavSource(
                    source.as_posix(), cache=False
                ).num_frames
                return total
        except:
            video = cv2.VideoCapture(source.as_posix())
            total = int(video.get(cv2.CAP_PROP_FRAME_COUNT))
            video.release()
    if is_vs or total < 1:
        total = frame_probe(source)

    return total


def frame_probe(source: Path):
    """
    Determines the total number of frames in a given input.

    Differentiates between a Vapoursynth script and standard video
    and delegates to vspipe or ffmpeg respectively.
    """
    if is_vapoursynth(source):
        return frame_probe_vspipe(source)

    return frame_probe_ffmpeg(source)
