"""
Toolbox - Project

Tools created for easier interfacing with Av1an for custom use.

"""
import os
import re
import subprocess
from pathlib import Path
from subprocess import run
from typing import Union, List, Iterable

from av1an.arg_parse import Args
from av1an.chunk import Chunk
from av1an.chunk.chunk_queue import load_or_gen_chunk_queue
from av1an.project import Project
from av1an.split import calc_split_locations, extra_splits
from av1an.startup.setup import startup_check



def create_default_project(options: Union[Iterable[str], str]) -> Project:
    """
    Accepts a list of Av1an arguments,
    or a single string input which will be considered the input file.
    """
    if isinstance(options, str):
        options = ['-i', options]
    elif not isinstance(options, Iterable):
        raise TypeError(f'options needs to be a string, or a list of strings, not "{type(options)}"')

    args = vars(Args().parser.parse_args(options))
    project = Project(args)
    startup_check(project)
    project.input = project.input[0]
    project.outputs_filenames()
    project.setup()
    return project


def gen_splits(project, method=None):
    """Generates splits for a project using given method, or project default if not specified."""
    if method is not None:
        project.split_method = method
    sc = calc_split_locations(project)
    sc = extra_splits(project, sc)

    return sc


def gen_chunks_queue(project, split_locations, method=None):
    if method is not None:
        project.chunk_method = method
    chunk_queue = load_or_gen_chunk_queue(project, project.resume, split_locations)
    return chunk_queue


def get_frames(video: Union[Chunk, Project]):
    if isinstance(video, Project):
        return video.get_frames()
    else:
        return video.frames


def _frame_probe_util(input_cmd):
    """
    Get frame count.
    Direct counting of frame count using ffmpeg. Slow, Precise.
    :param: source: Path to input file
    """
    cmd = [
        "ffmpeg",
        "-hide_banner",
        "-i",
        "-",
        "-map",
        "0:v:0",
        "-f",
        "null",
        "-",
    ]
    ffmpeg_gen_pipe = subprocess.Popen(
        input_cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        universal_newlines=True,
    )

    ffmpeg_pipe = subprocess.run(
        cmd,
        stdin=ffmpeg_gen_pipe.stdout,
        capture_output=True
    )

    text = ffmpeg_pipe.stderr.decode("utf-8") + ffmpeg_pipe.stdout.decode("utf-8")
    # print(text)
    matches = re.findall(
        r"frame=\s*([0-9]+)\s", text
    )
    # print(matches)
    total = int(matches[-1])

    return total


def frame_probe(video: Union[Chunk, Project]):
    if isinstance(video, Project):
        return video.get_frames()

    cmd = video.ffmpeg_gen_cmd
    total = _frame_probe_util(cmd)
    return total


# TODO: Add check for VFR, make Av1an create normalized stream
def quality_probe(chunk, project, q, params=None):

    pass


if __name__ == '__main__':
    test_input_str = 'test_destiny.mkv'
    os.chdir('..')
    os.chdir('..')
    test_project = create_default_project(['-i', test_input_str, '-xs', '600'])
    print(test_project.extra_split)
    print(test_project.input)
    print(os.getcwd())

    # test_project.threshold = 35
    test_splits = gen_splits(test_project)
    # test_splits2 = gen_splits(test_project, method='ffmpeg')
    # test_splits3 = gen_splits(test_project, 'pyscene')

    chunks = gen_chunks_queue(test_project, test_splits)
    print(chunks)
    print(chunks[0].ffmpeg_gen_cmd)
    frames = frame_probe(chunks[0])
    print(f'Expected {chunks[0].frames} vs actual {frames} ')
    print('-'*20)

    chunks = gen_chunks_queue(test_project, test_splits, method='vs_ffms2')
    print(chunks)
    print(chunks[0].ffmpeg_gen_cmd)
    frames = frame_probe(chunks[0])
    print(f'Expected {chunks[0].frames} vs actual {frames} ')
    print('-'*20)

    chunks = gen_chunks_queue(test_project, test_splits, method='hybrid')
    print(chunks)
    print(chunks[0].ffmpeg_gen_cmd)
    frames = frame_probe(chunks[0])
    print(f'Expected {chunks[0].frames} vs actual {frames} ')
    print('-'*20)

    chunks = gen_chunks_queue(test_project, test_splits, method='select')
    print(chunks)
    print(chunks[0].ffmpeg_gen_cmd)
    frames = frame_probe(chunks[0])
    print(f'Expected {chunks[0].frames} vs actual {frames} ')
    print('-'*20)









