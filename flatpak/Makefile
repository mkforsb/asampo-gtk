.PHONY: all sources build bundle clean

all: sources build bundle

sources:
	python3 flatpak-cargo-generator.py ../Cargo.lock

build:
	flatpak-builder --force-clean --user --install-deps-from=flathub --repo=repo --install builddir io.github.mkforsb.asampo_gtk.json

bundle:
	flatpak build-bundle repo asampo_gtk.flatpak io.github.mkforsb.asampo_gtk --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo

clean:
	rm -rf builddir .flatpak-builder repo generated-sources.json asampo_gtk.flatpak
