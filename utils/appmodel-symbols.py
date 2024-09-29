import re

with open("src/model/mod.rs") as fd:
    text = fd.read()

for matched in re.finditer(r"(?s)pub fn [^<\(]+", text):
    matched_text = matched.group().replace("\n", "")

    if matched_text.startswith("pub fn"):
        print(matched_text[7:])


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


for matched in re.finditer(r"delegate!\(([a-z_]+)", text):
    module = matched.group(1)
    s0 = matched.span()[0]
    delegates = text[s0 : s0 + balanced(text[s0:], "(", ")", 0) + 2]
    delegates = re.sub(r"(?ms)\n\s*as\s*", " as ", delegates)
    delegates = re.sub(r"(?ms)\s+as\s+", " as ", delegates)

    for matched in re.finditer(r"([a-z_]+)\(", delegates):
        fname = matched.group(1)
        s1 = matched.span()[0]
        s2 = s0 + s1 + balanced(text[s0 + s1 :], "(", ")", 0) + 1

        def maybe_index(text, needle):
            try:
                return text.index(needle)
            except:
                return None

        terminator = list(
            x
            for x in [
                maybe_index(text[s2:], ","),
                maybe_index(text[s2:], ")"),
                maybe_index(text[s2:], "\n"),
            ]
            if x is not None
        )[0]

        rename = re.search(r" as ([a-z_]+)", text[s2 : s2 + terminator])

        if rename is not None:
            print(f"{rename.group(1)}")
        else:
            print(f"{fname}")
