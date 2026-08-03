# Cross-build environment for the ARM handhelds, matching CI. zig supplies the
# glibc stubs, so the floor is a number in the target triple rather than whatever
# an old distro happens to ship — no archived apt suites involved.
FROM ubuntu:22.04

ARG ZIG_VERSION=0.15.2
ARG ZIGBUILD_VERSION=0.23.0
ARG SDL_VERSION=2.26.5+dfsg-1

RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      build-essential pkg-config ca-certificates curl git xz-utils binutils \
 && rm -rf /var/lib/apt/lists/*

RUN curl --retry 5 -fsSL \
      "https://ziglang.org/download/${ZIG_VERSION}/zig-x86_64-linux-${ZIG_VERSION}.tar.xz" \
    | tar -xJ -C /opt \
 && ln -s "/opt/zig-x86_64-linux-${ZIG_VERSION}/zig" /usr/local/bin/zig \
 && zig version

RUN curl --retry 5 -fsSL \
      "https://github.com/rust-cross/cargo-zigbuild/releases/download/v${ZIGBUILD_VERSION}/cargo-zigbuild-x86_64-unknown-linux-musl.tar.xz" \
    | tar -xJ -C /usr/local/bin --strip-components=1 \
      "cargo-zigbuild-x86_64-unknown-linux-musl/cargo-zigbuild" \
 && cargo-zigbuild --version

# Only for linking: the device loads its own SDL2. Ubuntu's is 2.0.20, which
# predates SDL_RenderGeometry, so unpack Debian's per-arch builds side by side.
RUN pool=http://deb.debian.org/debian/pool/main/libs/libsdl2 \
 && cd /tmp \
 && for arch in armhf arm64; do \
      mkdir -p "/opt/sdl2/${arch}"; \
      for pkg in libsdl2-2.0-0 libsdl2-dev; do \
        curl --retry 5 -fsSLO "$pool/${pkg}_${SDL_VERSION}_${arch}.deb" \
        && dpkg -x "${pkg}_${SDL_VERSION}_${arch}.deb" "/opt/sdl2/${arch}"; \
      done; \
    done \
 && rm -f /tmp/*.deb \
 && ls /opt/sdl2/armhf/usr/lib/arm-linux-gnueabihf/libSDL2*.so \
       /opt/sdl2/arm64/usr/lib/aarch64-linux-gnu/libSDL2*.so
