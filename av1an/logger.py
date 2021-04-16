#!/bin/env python

import inspect
import logging
import sys
from pathlib import Path


# Todo: Add self testing on startup
class Logger:
    def __init__(self):
        self.ready = False
        self.buffer = []
        self._logger: logging.Logger = logging.getLogger('Av1an')

        self._logger.setLevel(logging.DEBUG)

        self._formatter = logging.Formatter('[{asctime}]{message}', style="{", datefmt="%X")

    def set_path(self, file):
        for handler in self._logger.handlers[:]:
            handler.close()
            self._logger.removeHandler(handler)

        filehandler = logging.FileHandler(file, encoding='utf-8')
        filehandler.setFormatter(self._formatter)
        filehandler.setLevel(logging.DEBUG)
        self._logger.addHandler(filehandler)
        self.ready = True

    def close(self):
        for handler in self._logger.handlers:
            handler.close()
        self.ready = False

    def log(self, *info, include_caller=True):
        for i in info:
            if not self.ready:
                self.buffer.append(i)
                continue

            if self.buffer:
                for msg in self.buffer:
                    self._logger.info(msg)
                self.buffer.clear()

            if include_caller:
                cur_frame = inspect.currentframe()
                cal_frame = inspect.getouterframes(cur_frame, 2)
                parent_function = cal_frame[1][3]

                self._logger.info(f'[{parent_function}] {i}')
            else:
                self._logger.info(f' {i}')


# Creating logger
logger = Logger()
log_file = logger.set_path
log = logger.log


def unset_log():
    global logger
    logger.close()


def set_log(log_path: Path, temp):
    """Setting logging file location"""

    if log_path:
        log_path = Path(log_path)
        if log_path.suffix == "":
            log_path = log_path.with_suffix(".log")
        log_file(log_path)

    else:
        log_file(temp / "log.log")

    log(f"Av1an Started", f"Command:", f"{' '.join(sys.argv)}", include_caller=False)
