#!/usr/bin/python3

import json
import os
import os.path
import sys


def get_in(obj, *keys):
    try:
        result = obj[keys[0]]

        for key in keys[1:]:
            result = result[key]

        return result
    except:
        return None


def opt_iter(opt):
    if opt is not None:
        return opt
    else:
        return ()


def sampleset(data, name):
    for set in opt_iter(get_in(data, "V1", "samplesets")):
        if get_in(set, "BaseSampleSetV1", "name") == name:
            return set["BaseSampleSetV1"]


with open(sys.argv[1]) as fd:
    data = json.load(fd)


for sample in opt_iter(get_in(sampleset(data, "Trash"), "samples")):
    if os.path.isfile(sample["sample"]["BaseSampleV1"]["uri"][7:]):
        print(sample["sample"]["BaseSampleV1"]["uri"][7:])
        os.remove(sample["sample"]["BaseSampleV1"]["uri"][7:])
