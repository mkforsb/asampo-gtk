import re

with open("src/model/mod.rs") as fd:
    text = fd.read()

for matched in re.finditer(r"(?s)pub fn [^<\(]+|delegate!\(.+?\);", text):
    matched_text = matched.group().replace("\n", "")

    if matched_text.startswith("pub fn"):
        print(matched_text[7:])
    elif matched_text.startswith("delegate!"):
        if " as " in matched_text:
            print(re.search(r" as ([a-z_]+)", matched_text).group(1))
        else:
            print(re.search(r"delegate!\([a-z_]+,\s*([a-z_]+)", matched_text).group(1))

