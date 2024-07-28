#!/usr/bin/python

import os
import re

def template_children(text):
    return re.findall(r"([^\s]+): gtk::TemplateChild", text)

def source_files():
    return [f"{x[0]}/{y}" for x in os.walk("src") for y in x[2]]

def text(filename):
    with open(filename) as fd:
        return fd.read()

def source_text():
    return "".join(list(map(text, source_files())))

def vars():
    with open("src/view/mod.rs") as fd:
        return template_children(fd.read())

source = source_text()

for v in vars():
    if not f".{v}" in source:
        print(v)


