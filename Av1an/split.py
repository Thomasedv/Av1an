#!/bin/env python

import os
import subprocess
from pathlib import Path
from subprocess import PIPE, STDOUT
from typing import List

from .arg_parse import Args
from .ffmpeg import frame_probe, get_keyframes
from .aom_kf import aom_keyframes, AOM_KEYFRAMES_DEFAULT_PARAMS
from .logger import log
from .pyscene import pyscene
from .utils import terminate


def split_routine(args: Args, resuming: bool) -> List[int]:
    """
    Performs the split routine. Runs pyscenedetect/aom keyframes and adds in extra splits if needed

    :param args: the Args
    :param resuming: if the encode is being resumed
    :return: A list of frames to split on
    """
    scene_file = args.temp / 'scenes.txt'

    # if resuming, we already have the split file, so just read that
    if resuming:
        return read_scenes_from_file(scene_file)

    # determines split frames with pyscenedetect or aom keyframes
    split_locations = calc_split_locations(args)

    # add in extra splits if needed
    if args.extra_split:
        split_locations = extra_splits(args.input, split_locations, args.extra_split)

    # write scenes for resuming later if needed
    write_scenes_to_file(split_locations, scene_file)

    return split_locations


def write_scenes_to_file(scenes: List[int], scene_path: Path):
    """
    Writes a list of scenes to the a file

    :param scenes: the scenes to write
    :param scene_path: the file to write to
    :return: None
    """
    with open(scene_path, 'w') as scene_file:
        scene_file.write(','.join([str(x) for x in scenes]))


def read_scenes_from_file(scene_path: Path) -> List[int]:
    """
    Reads a list of split locations from a file

    :param scene_path: the file to read from
    :return: a list of frames to split on
    """
    with open(scene_path, 'r') as scene_file:
        scenes = scene_file.readline().strip().split(',')
        return [int(scene) for scene in scenes]


def segment(video: Path, temp: Path, frames: List[int]):
    """
    Uses ffmpeg to segment the video into separate files.
    Splits the video by frame numbers or copies the video if no splits are needed

    :param video: the source video
    :param temp: the temp directory
    :param frames: the split locations
    :return: None
    """

    log('Split Video\n')
    cmd = [
        "ffmpeg", "-hide_banner", "-y",
        "-i", video.absolute().as_posix(),
        "-map", "0:v:0",
        "-an",
        "-c", "copy",
        "-avoid_negative_ts", "1"
    ]

    if len(frames) > 0:
        cmd.extend([
            "-f", "segment",
            "-segment_frames", ','.join([str(x) for x in frames])
        ])
        cmd.append(os.path.join(temp, "split", "%05d.mkv"))
    else:
        cmd.append(os.path.join(temp, "split", "0.mkv"))
    pipe = subprocess.Popen(cmd, stdout=PIPE, stderr=STDOUT)
    while True:
        line = pipe.stdout.readline().strip()
        if len(line) == 0 and pipe.poll() is not None:
            break

    log('Split Done\n')


def extra_splits(video, split_locations: list, split_distance):
    log('Applying extra splits\n')
    # Get all keyframes of original video
    keyframes = get_keyframes(video)

    split_locs_with_start = split_locations[:]
    split_locs_with_start.insert(0, 0)

    split_locs_with_end = split_locations[:]
    split_locs_with_end.append(frame_probe(video))

    splits = list(zip(split_locs_with_start, split_locs_with_end))
    for i in splits:
        # Getting distance between splits
        distance = (i[1] - i[0])

        if distance > split_distance:
            # Keyframes that between 2 split points
            candidates = [k for k in keyframes if i[1] > k > i[0]]

            if len(candidates) > 0:

                # Getting number of splits that need to be inserted
                to_insert = min((i[1] - i[0]) // split_distance, (len(candidates)))
                for k in range(0, to_insert):

                    # Approximation of splits position
                    aprox_to_place = (((k + 1) * distance) // (to_insert + 1)) + i[0]

                    # Getting keyframe closest to approximated
                    key = min(candidates, key=lambda x: abs(x - aprox_to_place))
                    split_locations.append(key)
    result = [int(x) for x in sorted(split_locations)]
    log(f'Split distance: {split_distance}\nNew splits:{len(result)}\n')
    return result


def calc_split_locations(args: Args) -> List[int]:
    """
    Determines a list of frame numbers to split on with pyscenedetect or aom keyframes

    :param args: the Args
    :return: A list of frame numbers
    """
    # inherit video params from aom encode unless we are using a different encoder, then use defaults
    aom_keyframes_params = args.video_params if (args.encoder == 'aom') else AOM_KEYFRAMES_DEFAULT_PARAMS

    if args.scenes == '0':
        log('Skipping scene detection\n')
        return []

    sc = []

    if args.scenes:
        args.scenes = Path(args.scenes)
        if args.scenes.exists():
            # Read stats from CSV file opened in read mode:
            log('Using Saved Scenes\n')
            return read_scenes_from_file(args.scenes)

    # Splitting using PySceneDetect
    if args.split_method == 'pyscene':
        log(f'Starting scene detection Threshold: {args.threshold}, Min_scene_length: {args.min_scene_len}\n')
        try:
            sc = pyscene(args.input, args.threshold, args.min_scene_len)
        except Exception as e:
            log(f'Error in PySceneDetect: {e}\n')
            print(f'Error in PySceneDetect{e}\n')
            terminate()

    # Splitting based on aom keyframe placement
    elif args.split_method == 'aom_keyframes':
        stat_file = args.temp / 'keyframes.log'
        sc = aom_keyframes(args.input, stat_file, args.min_scene_len, args.ffmpeg_pipe, aom_keyframes_params)
    else:
        print(f'No valid split option: {args.split_method}\nValid options: "pyscene", "aom_keyframes"')
        terminate()

    # Write scenes to file

    if args.scenes:
        write_scenes_to_file(sc, args.scenes)

    return sc
