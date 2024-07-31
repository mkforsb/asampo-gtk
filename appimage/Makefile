.PHONY: all

all:
	docker build -t asampo-appimage .
	docker run --rm --mount type=bind,src=.,dst=/home/user/out asampo-appimage
