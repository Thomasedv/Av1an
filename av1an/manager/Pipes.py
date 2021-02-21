from collections import deque

from av1an.chunk import Chunk
from av1an.encoder import ENCODERS
from av1an.logger import log
from av1an.project import Project


def process_pipe(pipe, chunk: Chunk):
    encoder_history = deque(maxlen=20)

    while True:
        if chunk.cancel:
            pipe.kill()
            return

        line = pipe.stdout.readline().strip()
        if len(line) == 0 and pipe.poll() is not None:
            break
        if len(line) == 0:
            continue
        if line:
            encoder_history.append(line)

    if pipe.returncode != 0 and pipe.returncode != -2:
        msg1 = f'Encoder encountered an error: {pipe.returncode}'
        msg2 = f'Chunk: {chunk.index}' + \
               '\n'.join(encoder_history)
        log(msg1, msg2)
        print(f'::{msg1}\n::{msg2}')
        raise Exception("Error in processing pipe")


def process_encoding_pipe(pipe, encoder, counter, chunk: Chunk):
    encoder_history = deque(maxlen=20)
    frame = 0
    enc = ENCODERS[encoder]
    while True:
        if chunk.cancel:
            pipe.kill()
            return

        line = pipe.stdout.readline().strip()
        if len(line) == 0 and pipe.poll() is not None:
            break

        if len(line) == 0:
            continue

        match = enc.match_line(line)

        if match:
            new = int(match.group(1))
            if new > frame:
                counter.update(new - frame)
                frame = new

        if line:
            encoder_history.append(line)

    if pipe.returncode != 0 and pipe.returncode != -2:  # -2 is Ctrl+C for aom
        if pipe.returncode == 3221225786:
            raise KeyboardInterrupt('User stopped')

        msg1 = f'Encoder encountered an error: {pipe.returncode}'
        msg2 = f'Chunk: {chunk.index}'
        msg3 = '\n'.join(encoder_history)
        log(msg1, msg2, msg3)
        print(f'::{msg1}\n::{msg2}\n::{msg3}')
        raise Exception("Error in processing encoding pipe")


def tqdm_bar(a: Project, c: Chunk, encoder, counter, frame_probe_source,
             passes, current_pass):
    enc = ENCODERS[encoder]
    pipe = enc.make_pipes(a, c, passes, current_pass, c.output)

    if encoder in ('aom', 'vpx', 'rav1e', 'x265', 'x264', 'vvc', 'svt_av1'):
        process_encoding_pipe(pipe, encoder, counter, c)

    if encoder in ('svt_vp9'):
        # SVT-VP9 is special
        process_pipe(pipe, c)
        counter.update(frame_probe_source // passes)
