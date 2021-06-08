"""
Toolbox - Project

Tools created for easier interfacing with Av1an for custom use.

"""
import os
from pathlib import Path
from typing import Union, List, Iterable

from av1an.arg_parse import Args
from av1an.project import Project
from av1an.split import calc_split_locations
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
    print(sc)
    return sc


if __name__ == '__main__':
    test_input_str = 'death_sample.mkv'
    os.chdir('..')
    os.chdir('..')
    test_project = create_default_project(['-i', test_input_str])

    print(test_project.input)
    print(os.getcwd())
    test_project.threshold = 20
    # test_splits = gen_splits(test_project)
    test_splits2 = gen_splits(test_project, method='ffmpeg')
    # test_splits3 = gen_splits(test_project, 'pyscene')


