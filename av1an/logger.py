#!/bin/env python

import inspect
import logging
import sys
import time
from pathlib import Path


# Todo: Add self testing on startup
class Logger:
    def __init__(self):
        self.ready = False
        self.buffer = []
        self._logger = logging.getLogger('Av1an')
        self._logger.setLevel(logging.DEBUG)

        self._formatter = logging.Formatter('{message}', style="{")

    def set_path(self, file):
        for handler in self._logger.handlers[:]:
            handler.close()
            self._logger.removeHandler(handler)

        filehandler = logging.FileHandler(file, encoding='utf-8')
        filehandler.setFormatter(self._formatter)
        filehandler.setLevel(logging.DEBUG)
        self._logger.addHandler(filehandler)
        self.ready = True

    def log(self, *info, include_caller=True):
        cur_frame = inspect.currentframe()
        cal_frame = inspect.getouterframes(cur_frame, 2)
        parent_function = cal_frame[1][3]

        for i in info:
            if not self.ready:
                self.buffer.append((f'[{time.strftime("%X")}]', f'[{parent_function}]' * include_caller, i))
                continue

            if self.buffer:
                for ts, pf, msg in self.buffer:
                    self._logger.info(f'{ts} {pf} {msg}')
                self.buffer.clear()

            if include_caller:
                self._logger.info(f'[{time.strftime("%X")}] [{parent_function}] {i}')
            else:
                self._logger.info(f'[{time.strftime("%X")}] {i}')


# Creating logger
logger = Logger()
log_file = logger.set_path
log = logger.log


def set_log(log_path: Path, temp):
    """Setting logging file location"""

    if log_path:
        log_path = Path(log_path)
        if log_path.suffix == '':
            log_path = log_path.with_suffix('.log')
        log_file(log_path)

    else:
        log_file(temp / 'log.log')

    log(f"Av1an Started", f"Command:", f"{' '.join(sys.argv)}", include_caller=False)
