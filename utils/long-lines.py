#!/usr/bin/python3

import os

def source_files():
    return [f"{x[0]}/{y}" for x in os.walk("src") for y in x[2]]

def text(filename):
    with open(filename) as fd:
        return fd.read()

files_text = {file: text(file).split("\n") for file in source_files()}
files_line_lens = {file: {num: len(line) for num,line in enumerate(files_text[file])}
                   for file in files_text}

for file, lines in sorted(files_line_lens.items()):
    for line, length in sorted(lines.items()):
        if length > 100:
            print(f"{file}:{line} length {length}")
