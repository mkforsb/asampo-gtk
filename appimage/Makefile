.PHONY: all clean

all:
	docker build -t asampo-appimage .
	docker run --rm --mount type=bind,src=.,dst=/home/user/out asampo-appimage

clean:
	rm -f asampo-x86_64.AppImage
