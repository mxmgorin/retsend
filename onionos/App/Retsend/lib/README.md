# Bundled libraries

The Miyoo Mini's panel hangs off the SigmaStar display pipeline, and no upstream
SDL2 can put pixels on it; the port needs a build carrying the `Mini` video
driver. So every SDL2 app here ships its own copy and preloads it, and these
files are ours.

## libSDL2-2.0.so.0

Built from our fork [mxmgorin/sdl2](https://github.com/mxmgorin/sdl2), branch
`miyoomini-flip` (`03338482`), which is [steward-fu/sdl2](https://github.com/steward-fu/sdl2)
at `0631abc8` plus one commit, made in the upstream Docker recipe,
`mini_toolchain-v1.0`, `make cfg && make sdl2`. It reports itself as
`libSDL2-2.0.so.0.18.2`, so SDL 2.0.18 — which is the floor retsend builds
against anyway, for `SDL_RenderGeometry`.

Our commit is `feat(mini): size the framebuffer and textures from the panel`, and
it exists because the Flip's panel is 752x560 while these drivers assumed the
Mini's 640x480. Three things in it:

- the framebuffer size is parsed out of `fbset`'s mode line rather than found by
  `strstr(buf, "752")`, so a panel reporting 750 or anything else is read
  correctly;
- `SDL_GetDesktopDisplayMode` answers with that size — the driver used to leave
  it zeroed, so an app had no way to ask how big the screen was;
- the renderer's `max_texture_width/height` come from the framebuffer instead of
  a hardcoded 640x480, which is what stopped a window that fills a Flip from
  having a texture to draw into.

On a Mini or Mini+ every one of those resolves to the same 640x480 it always
was. The exported symbol set is unchanged (805 symbols, checked both ways) and
so is `DT_NEEDED`.

Previous packages carried a prebuilt copy taken from the `Sonic Mania` port of
the OnionOS Ports-Collection, built from
[XK9274/sdl2_miyoo](https://github.com/XK9274/sdl2_miyoo)'s `snow-s-mania`
branch. It was replaced because that branch advertises its texture formats as

    .num_texture_formats = 2,
    .texture_formats = { [0] = SDL_PIXELFORMAT_RGB565, [2] = SDL_PIXELFORMAT_ARGB8888 },

where index `[2]` should be `[1]`. With two formats declared, SDL reads slots 0
and 1, finds RGB565 and a zero, and can offer no 32-bit format at all: the frame
retsend hands over — one 640x480 texture per repaint — was converted down to
RGB565 on the CPU every time, and the panel showed a 16-bit version of a UI
whose flat fills and thin dividers band under it. The same table in this build
reads `[0] = RGB565, [1] = ARGB8888`, which is what the driver actually supports,
so the frame stays 32-bit for the SoC's blitter. That branch also makes
`SELECT + R1` a screen-scaling hotkey and eats the `R1` press, which pages a list
here.

Note this is an *altered* version of SDL, which its licence requires be said
plainly: the `Mini` video, render and audio drivers are third-party additions
and not part of upstream SDL.

### What the launcher has to say to it

The video driver is named `Mini` and its render driver `Miyoo Mini`, so
`SDL_VIDEODRIVER=Mini`. `SDL_RENDER_DRIVER` has to name the render driver as
well: SDL's own `software` driver is listed ahead of it and comes up first, and
what it draws through this video driver's window framebuffer never reaches the
panel. The pad arrives as key presses either way, but layout detection keys off
the driver name `mmiyoo`, so the launcher asks for it outright with
`RETSEND_KEYMAP=miyoo`.

Only the render driver's texture copy draws anything at all — its geometry and
fill hooks return without touching the screen — so `RETSEND_BLIT=1` is not a
tuning knob on this device but the only path: egui is drawn by SDL's software
renderer into an offscreen surface, and the panel gets one texture copy of it
per frame.

## libEGL.so

A 7 KB placeholder from the Ports-Collection build. `libSDL2` names `libEGL` in
`DT_NEEDED` and references a handful of `egl*` functions, but the library is
configured with `--disable-video-opengl`, `--disable-video-opengles` and
`--disable-video-opengles2` and never calls them; binding is lazy, so a
placeholder that exports nothing is enough. Its licence is not stated at the
source, and it exists only to satisfy the loader. Swap it for the device's own
EGL, or for `swiftshader/build/libEGL.so` from the build above, if that
provenance is not good enough for a redistribution you intend.

## libjson-c.so.5

json-c, from the same Ports-Collection bundle. `libSDL2`'s audio driver reads
`/appconfigs/system.json` for the system volume; retsend opens no audio device,
but the symbols are resolved regardless.

## fallback/libGLESv2.so, fallback/libshmvar.so

`libSDL2` names both in `DT_NEEDED` and calls into neither: it is configured
`--disable-video-opengles`, and `nm -D` shows no `gl*` or `shmvar*` among its
undefined symbols. An OnionOS card carries them, an Allium card does not, and on
one the loader stops before `main`:

    ./retsend: error while loading shared libraries: libGLESv2.so: cannot open
    shared object file: No such file or directory

So these are empty shared objects with the right `SONAME`, built here with
`zig cc -shared -target arm-linux-gnueabihf.2.28` from a `.c` file holding one
comment. They are ours, under this package's licence, and the launcher puts
`lib/fallback` at the *end* of `LD_LIBRARY_PATH`: where the real libraries exist
they are found first, and these fill the gap where they do not.

    5822e549fae04bdf4ea2af7efb4f39f15c398066d3107fa576467b0d88947145  libGLESv2.so
    b92c2058527c31757298112a563a44c987b48dc35c64ab2ba41ac9c5183057c9  libshmvar.so

## Not shipped

`libSDL2_image-2.0.so.0` and `libSDL2_ttf-2.0.so.0` were in the previous bundle
only because that build named them in `DT_NEEDED` for an overlay feature retsend
never used. This one does not, so they are gone, and with them `libpng16`,
`libz`, `libfreetype` and `libbz2`. The `libmi_*` SoC libraries are the
firmware's; `libGLESv2` is an OnionOS card's, and `fallback/` covers the cards
without one.

    adfaba3ed88c5e5acd521384f047bc369f52578f01d0b6b39fc7591e94e93944  libEGL.so
    81c07c104c025ff61586f53a26245d37e5006db5961e55a88e7a648590db5cd8  libSDL2-2.0.so.0
    db7ad1f59cbac5a23aad9dd7eba87322b5921c486097c1140b24f2d82af28892  libjson-c.so.5

## Licences

`libSDL2-2.0.so.0` is Simple DirectMedia Layer, under the zlib licence; the
corresponding source is the fork and commit named above.

> Simple DirectMedia Layer
> Copyright (C) 1997-2024 Sam Lantinga <slouken@libsdl.org>
>
> This software is provided 'as-is', without any express or implied warranty. In
> no event will the authors be held liable for any damages arising from the use
> of this software.
>
> Permission is granted to anyone to use this software for any purpose, including
> commercial applications, and to alter it and redistribute it freely, subject to
> the following restrictions:
>
> 1. The origin of this software must not be misrepresented; you must not claim
>    that you wrote the original software. If you use this software in a product,
>    an acknowledgment in the product documentation would be appreciated but is
>    not required.
> 2. Altered source versions must be plainly marked as such, and must not be
>    misrepresented as being the original software.
> 3. This notice may not be removed or altered from any source distribution.

The `Mini` drivers added on top of it are LGPL-2.1, (C) 2025 Steward Fu, as
their source files state, with our one commit on top; they are built into this
shared library, and the source is the fork named above.

`libjson-c.so.5` is json-c, under the MIT licence:

> Copyright (c) 2009-2012 Eric Haszlakiewicz
> Copyright (c) 2004, 2005 Metaparadigm Pte Ltd
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of
> this software and associated documentation files (the "Software"), to deal in
> the Software without restriction, including without limitation the rights to
> use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
> of the Software, and to permit persons to whom the Software is furnished to do
> so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

`libEGL.so` is a 7 KB stub shipped in the Ports-Collection bundle; its licence
is not stated there, and it exists only to satisfy `DT_NEEDED`.
