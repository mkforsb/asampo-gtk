#!/usr/bin/python3

from collections import namedtuple
import re

def balanced(text, open, close, depth):
    pos = 0
    length = len(text)
    rt_depth = depth

    while pos < length:
        if text[pos] == open:
            rt_depth += 1
        elif text[pos] == close:
            rt_depth -= 1

            if rt_depth == 0:
                return pos
        
        pos += 1

def text(filename):
    with open(filename) as fd:
        return fd.read()

def line(code, offset):
    return code[:offset].count("\n")

def tests(code):
    result = []

    for match in re.finditer(r"(?m)^\s*#\[test", code):
        fnheader = re.search(r"(?ms)^\s*fn.+?{", code[match.span()[1]:])
        bodylen = balanced(code[match.span()[1] + fnheader.span()[1] - 1:], "{", "}", 0)
        result.append({
            "start": match.span()[0],
            "end": match.span()[1] + fnheader.span()[1] + bodylen,
            "start_line": line(code, match.span()[0] + 1),
            "end_line": line(code, match.span()[1] + fnheader.span()[1] + bodylen),
            "code": code[match.span()[0]:match.span()[1] + fnheader.span()[1] + bodylen],
            "header": code[match.span()[1] + fnheader.span()[0]:match.span()[1] + fnheader.span()[1]],
        })

    return result

def first(xs):
    if len(xs) > 0:
        return xs[0]
    else:
        return None

textentry = namedtuple("textentry", ["linenum", "text"])
blankentry = namedtuple("blankentry", ["linenum"])
testentry = namedtuple("testentry", ["linenum_start", "header", "code"])
testgroup = namedtuple("testgroup", ["linenum_start", "tests"])

def classify(code):
    result = []
    code_tests = tests(code)

    i = 0
    lines = code.split("\n")

    while i < len(lines):
        test = first([x for x in code_tests if x["start_line"] == i])
        
        if test is None:
            if re.search(r"\S", lines[i]):
                result.append(textentry(i, lines[i]))
            else:
                result.append(blankentry(i))

            i += 1
        else:
            result.append(testentry(i, test["header"], test["code"]))
            i = test["end_line"] + 1
    
    while True:
        changed = False

        for i in range(len(result)):
            if i + 2 < len(result)\
            and type(result[i]) == testentry\
            and type(result[i+1]) == blankentry\
            and type(result[i+2]) == testentry:
                del result[i+1]
                changed = True
                break

            i += 1

        if not changed:
            break

    i = 0

    while True:
        if i > 0 and type(result[i-1]) == testgroup:
            group = i-1
        else:
            group = None

        if type(result[i]) == testentry:
            if group is None:
                result.insert(i, testgroup(i, [result[i]]))
                del result[i + 1]
            else:
                result[group] = testgroup(result[group].linenum_start, result[group].tests + [result[i]])
                del result[i]
                i -= 1
        else:
            group = None

        i += 1

        if i >= len(result):
            break

    return result


def render(stuff):
    for entry in stuff:
        match entry:
            case textentry(_, text):
                print(text)
            case blankentry():
                print()
            case testgroup(_, tests):
                for test in sorted(tests, key=lambda t: t.header):
                    print(test.code)



if __name__ == "__main__":
    import sys
    import os

    if len(sys.argv) < 2 or not os.path.exists(sys.argv[1]):
        print(f"usage: {sys.argv[0]} path/to/code.rs")
    else:
        code = text(sys.argv[1])
        render(classify(code))

