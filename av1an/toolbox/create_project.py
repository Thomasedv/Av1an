"""
Toolbox - Project

Tools created for easier interfacing with Av1an for custom use.

"""
import os
import re
import subprocess
import time
from pathlib import Path
from subprocess import run
from typing import Union, List, Iterable, Dict

from av1an.arg_parse import Args
from av1an.chunk import Chunk
from av1an.chunk.chunk_queue import load_or_gen_chunk_queue
from av1an.logger import set_log, unset_log
from av1an.project import Project
from av1an.split import calc_split_locations, extra_splits
from av1an.startup.setup import startup_check
from av1an.target_quality import TargetQuality


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
    project.workers = 1  # Required to be set, user selected number
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


target_quality: Dict[Project, TargetQuality] = {}


# TODO: Add intercept logger, which takes over logging for func
# TODO: Add check for VFR, make Av1an create normalized stream
def quality_probe(chunk, project, params=None, rate=None):
    tq_handler = target_quality.get(project, None)
    if tq_handler is None:
        tq_handler = TargetQuality(project)

    project.probing_rate = rate if rate is not None else 4
    chunk.probing_rate = rate if rate is not None else 4
    project.n_subsample = chunk.probing_rate

    return tq_handler.per_shot_target_quality(chunk)


def new_quality_probe(chunk, project, rate=None, nsample=None):
    tq_handler = target_quality.get(project, None)
    if tq_handler is None:
        tq_handler = TargetQuality(project)

    project.probing_rate = rate if rate is not None else 4
    project.n_subsample = nsample if nsample is not None else project.probing_rate

    return tq_handler.new_per_shot_target_quality(chunk)


def vmaf_probe(chunk, project, q, params=None):
    pass


if __name__ == '__main__':
    # test_input_str = 'test_destiny.mkv'
    # test_input_str = 'death_sample.mkv'
    os.chdir('..')
    os.chdir('..')
    for test_input_str in ['test_destiny.mkv', r'C:\Users\thoma\Downloads\00087.mp4', 'death_sample.mkv']:
        test_project = create_default_project(['-i', test_input_str, '-xs', '300', '--vmaf-path',
                                               'vmaf_v0.6.1.json', '--target-quality', '95'])

        # print(test_project.extra_split)
        print(test_project.input)
        print(os.getcwd())
        set_log(test_project.logging, test_project.temp)

        # test_project.threshold = 35
        test_splits = gen_splits(test_project)
        # print(test_splits)

        # test_splits = [250, 402, 527, 724, 886, 972, 1065, 1212, 1425, 1512, 1691, 1937, 2053, 2113, 2195, 2418, 2577, 2640, 2768]
        # test_splits2 = gen_splits(test_project, method='ffmpeg')
        # test_splits3 = gen_splits(test_project, 'pyscene')

        chunks = gen_chunks_queue(test_project, test_splits)
        for chunk in chunks:
            new_rate = 2
            print(f'Chunk {chunk.index} -- {chunk.name}')

            start = time.time()
            q_new = new_quality_probe(chunk, test_project, rate=4, nsample=4)
            end_new = time.time() - start
            print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {4} | nsample {4}')

            start = time.time()
            q_new = new_quality_probe(chunk, test_project, rate=4, nsample=2)
            end_new = time.time() - start
            print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {4} | nsample {2}')

            start = time.time()
            q_new = new_quality_probe(chunk, test_project, rate=3, nsample=3)
            end_new = time.time() - start
            print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {3} | nsample {3}')

            # start = time.time()
            # q_new = new_quality_probe(chunk, test_project, rate=new_rate, nsample=4)
            # end_new = time.time() - start
            # print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {new_rate} | nsample {4}')

            # start = time.time()
            # q_new = new_quality_probe(chunk, test_project, rate=1, nsample=2)
            # end_new = time.time() - start
            # print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {1} | nsample {2}')

            start = time.time()
            q_new = new_quality_probe(chunk, test_project, rate=new_rate)
            end_new = time.time() - start
            print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {new_rate} | nsample {new_rate}')

            start = time.time()
            q_new = new_quality_probe(chunk, test_project, rate=new_rate, nsample=1)
            end_new = time.time() - start
            print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {new_rate} | nsample {1}')

            # TLDR: Results: Rate 4
            # New target_quality: Q: 50 Time 146.27769112586975
            # [23:40:47][log_probes] Chunk: 00019, Rate: 4, Fr: 262
            # [23:40:47][log_probes] Probes: (86.15, 55), (92.29, 52), (93.94, 51), (100.0, 35)
            # [23:40:47][log_probes] Target Q: 50 VMAF: 94.27

            start = time.time()
            q_old = quality_probe(chunk, test_project, rate=new_rate)
            end_old = time.time() - start
            print(f'Old target_quality: Q: {q_old} Time {end_old:.1f} | Rate {new_rate} | nsample {new_rate}')

            start = time.time()
            q_old = quality_probe(chunk, test_project, rate=4)
            end_old = time.time() - start
            print(f'Old target_quality: Q: {q_old} Time {end_old:.1f} | Rate {4} | nsample {4}')

            start = time.time()
            q_old = quality_probe(chunk, test_project, rate=1)
            end_old = time.time() - start
            print(f'Old target_quality: Q: {q_old} Time {end_old:.1f} | Rate {1} | nsample {1}')

            # start = time.time()
            # q_new = new_quality_probe(chunk, test_project, rate=1)
            # end_new = time.time() - start
            # print(f'New target_quality: Q: {q_new} Time {end_new:.1f} | Rate {1} | nsample {1}')
            # print('------' * 5)
        unset_log()
    # TLDR: Results: Rate 4
    # Old target_quality: Q: 45 Time 296.14592003822327
    # [23:21:27][log_probes] Chunk: 00019, Rate: 4, Fr: 262
    # [23:21:27][log_probes] Probes: (84.39, 55), (88.62, 52), (90.75, 50), (100.0, 35)
    # [23:21:27][log_probes] Target Q: 45 VMAF: 94.68

    # setps + select and setps rate in vmaf
    # New target_quality: Q: 50 Time 146.27769112586975
    # Old target_quality: Q: 45 Time 291.09849548339844

    # setps + select and setps rate+1 in vmaf

    # print(chunks)
    # print(chunks[0].ffmpeg_gen_cmd)
    # frames = frame_probe(chunks[0])
    # print(f'Expected {chunks[0].frames} vs actual {frames} ')
    # print('-'*20)
    #
    # chunks = gen_chunks_queue(test_project, test_splits, method='vs_ffms2')
    # print(chunks)
    # print(chunks[0].ffmpeg_gen_cmd)
    # frames = frame_probe(chunks[0])
    # print(f'Expected {chunks[0].frames} vs actual {frames} ')
    # print('-'*20)
    #
    # chunks = gen_chunks_queue(test_project, test_splits, method='hybrid')
    # print(chunks)
    # print(chunks[0].ffmpeg_gen_cmd)
    # frames = frame_probe(chunks[0])
    # print(f'Expected {chunks[0].frames} vs actual {frames} ')
    # print('-'*20)
    #
    # chunks = gen_chunks_queue(test_project, test_splits, method='select')
    # print(chunks)
    # print(chunks[0].ffmpeg_gen_cmd)
    # frames = frame_probe(chunks[0])
    # print(f'Expected {chunks[0].frames} vs actual {frames} ')
    # print('-'*20)
