import re

data = open("resources/asampo.ui").read()

for x in re.findall(r'class="(.+?)" id="(.+?)"', data):
    if x[1][0] == "-":
        continue

    print(f"    #[template_child(id = \"{x[1]}\")]")
    print(f"    pub {x[1].replace('-', '_')}: gtk::TemplateChild<gtk::{x[0][3:]}>,")
    print()
