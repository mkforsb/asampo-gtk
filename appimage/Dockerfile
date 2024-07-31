FROM debian:oldstable

RUN apt update && apt install -y --no-install-recommends build-essential ca-certificates \
    cmake curl doxygen file git graphviz libcairo2-dev libglx-dev libpulse-dev libxcursor-dev \
    libxdamage-dev libxi-dev libxinerama-dev libxkbcommon-dev libxml2-dev libxrandr-dev \
    pkg-config python3 python3-setuptools shared-mime-info wget xmlto xsltproc

RUN useradd -ms /bin/bash user
USER user
WORKDIR /home/user

RUN wget "https://github.com/mesonbuild/meson/archive/refs/tags/1.5.1.tar.gz" \
    && tar xvf 1.5.1.tar.gz

RUN wget "https://github.com/ninja-build/ninja/archive/refs/tags/v1.12.1.tar.gz" \
    && tar xvf v1.12.1.tar.gz

RUN wget "https://gitlab.freedesktop.org/glvnd/libglvnd/-/archive/v1.7.0/libglvnd-v1.7.0.tar.gz" \
    && tar xvf libglvnd-v1.7.0.tar.gz

RUN wget "https://gitlab.gnome.org/GNOME/gtk/-/archive/4.12.5/gtk-4.12.5.tar.gz" \
    && tar xvf gtk-4.12.5.tar.gz

RUN wget "https://gitlab.freedesktop.org/wayland/wayland/-/archive/1.22.0/wayland-1.22.0.tar.gz" \
    && tar xvf wayland-1.22.0.tar.gz

RUN wget "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" \
    && chmod +x appimagetool-x86_64.AppImage \
    && ./appimagetool-x86_64.AppImage --appimage-extract

RUN wget "https://raw.githubusercontent.com/AppImageCommunity/pkg2appimage/199fc9f768b35ad6166109299e5cf3832adeb5d5/excludelist"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh \
    && chmod +x /tmp/rustup.sh \
    && /tmp/rustup.sh -y

RUN git clone https://github.com/mkforsb/asampo-gtk.git

RUN cd ninja-1.12.1 && cmake . && make
USER root
RUN cd ninja-1.12.1 && make install
RUN cd meson-1.5.1 && python3 setup.py install

USER user
RUN cd wayland-1.22.0 && meson setup build && meson compile -C build
RUN cd libglvnd-v1.7.0 && meson setup build && meson compile -C build

USER root
RUN cd wayland-1.22.0 && meson install -C build
RUN cd libglvnd-v1.7.0 && meson install -C build
RUN ldconfig

USER user
RUN cd gtk-4.12.5 \
    && meson setup build -Dprefix=/usr -Dbuildtype=release -Dmedia-gstreamer=disabled \
                         -Dbuild-demos=false -Dbuild-examples=false \
                         -Dbuild-tests=false -Dbuild-testsuite=false \
    && meson compile -C build

USER root
RUN cd gtk-4.12.5 && meson install -C build

USER user
RUN . .cargo/env && cd asampo-gtk && cargo build --release

RUN mkdir -p asampo-appimage/usr/bin
COPY --chown=user:user AppRun asampo.desktop asampo.png asampo.svg /home/user/asampo-appimage

RUN cp asampo-gtk/target/release/asampo-gtk /home/user/asampo-appimage/usr/bin

RUN mkdir -p asampo-appimage/usr/share/glib-2.0
RUN cp -R /usr/share/glib-2.0/schemas asampo-appimage/usr/share/glib-2.0

RUN bash -c "ldd asampo-appimage/usr/bin/asampo-gtk | grep -Po '=> [^\s]+' | cut -b5- > deps.txt"

RUN bash -c "cat excludelist | grep -Pv '^#' | grep -P '.' | sed -E 's/\s+#.+//g' > excludes.txt"

RUN bash -c 'for line in $(cat deps.txt); do \
               lib=$(basename $line); \
               if [ "$(cat excludes.txt | grep $lib)" = "" ]; then \
                   mkdir -vp asampo-appimage/$(dirname "$line"); \
                   cp -v /"$line" asampo-appimage/"$line"; \
               fi; \
            done'

RUN ./squashfs-root/AppRun --no-appstream asampo-appimage

USER root
COPY docker-entry.sh /usr/bin
ENTRYPOINT ["docker-entry.sh"]
