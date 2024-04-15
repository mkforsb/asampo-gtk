import re

data = open("resources/asampo.ui").read()

for x in re.findall(r'class="(.+?)" id="(.+?)"', data):
    if "-" in x[1]:
        continue

    print("    #[template_child]")
    print(f"    pub {x[1]}: gtk::TemplateChild<gtk::{x[0][3:]}>,")
    print()
