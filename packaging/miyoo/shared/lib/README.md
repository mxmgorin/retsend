# Bundled libraries

No upstream SDL2 can draw on the Miyoo Mini's SigmaStar panel; it takes a build
with the `Mini` video driver. Every SDL2 app here ships its own and preloads it.

## libSDL2-2.0.so.0

- **Source:** [mxmgorin/sdl2](https://github.com/mxmgorin/sdl2), branch `miyoomini-flip`
  (`03338482`) = [steward-fu/sdl2](https://github.com/steward-fu/sdl2) `0631abc8` + one commit.
- **Build:** upstream Docker recipe `mini_toolchain-v1.0`, `make cfg && make sdl2`.
  Reports `libSDL2-2.0.so.0.18.2` (SDL 2.0.18, retsend's floor for `SDL_RenderGeometry`).
- **Altered SDL**, as its licence requires stating: the `Mini` video, render and
  audio drivers are third-party, not upstream SDL.

Our commit, `feat(mini): size the framebuffer and textures from the panel`, for
the Flip's 752x560 panel (the drivers assumed 640x480):

- framebuffer size parsed from `fbset`'s mode line, not `strstr(buf, "752")`;
- `SDL_GetDesktopDisplayMode` returns that size instead of zeros;
- `max_texture_width/height` follow the framebuffer instead of 640x480.

On a Mini or Mini Plus all three still resolve to 640x480. Exported symbols (805)
and `DT_NEEDED` are unchanged.

It replaces the prebuilt from the OnionOS Ports-Collection `Sonic Mania` port
([XK9274/sdl2_miyoo](https://github.com/XK9274/sdl2_miyoo), `snow-s-mania`): that
build declares its second texture format at index `[2]` instead of `[1]`, so SDL
offered no 32-bit format and every frame was converted to RGB565, and it binds
`SELECT + R1` to screen scaling, eating R1.

### What the launcher sets

- `SDL_VIDEODRIVER=Mini` and `SDL_RENDER_DRIVER="Miyoo Mini"`: SDL's `software`
  render driver comes first otherwise, and its output never reaches the panel.
- `RETSEND_BLIT=1`: only the render driver's texture copy draws; its geometry and
  fill hooks are no-ops.
- `RETSEND_KEYMAP=miyoo`: the pad arrives as keys, and layout detection keys off
  the driver name `mmiyoo`, not `Mini`.

## libEGL.so

A 7 KB placeholder from the Ports-Collection bundle. `libSDL2` lists `libEGL` in
`DT_NEEDED` but is built without GL and never calls it; binding is lazy, so an
empty library satisfies the loader. Its licence is not stated at the source.

## libjson-c.so.5

json-c from the same bundle. `libSDL2`'s audio driver links it to read the system
volume; retsend opens no audio, but the loader resolves the symbols.

## fallback/libGLESv2.so, fallback/libshmvar.so

`libSDL2` lists both in `DT_NEEDED` and calls neither. OnionOS cards carry them,
Allium cards do not, and without them the loader fails before `main`. These are
empty libraries with the right `SONAME`, built with
`zig cc -shared -target arm-linux-gnueabihf.2.28`, under this package's licence.
The launchers put `lib/fallback` last on `LD_LIBRARY_PATH`, so real ones win.

## Not shipped

`libSDL2_image`, `libSDL2_ttf` and their `libpng16`, `libz`, `libfreetype`,
`libbz2`: the previous build needed them, this one does not. `libmi_*` belong to
the firmware.

## Checksums

    81c07c104c025ff61586f53a26245d37e5006db5961e55a88e7a648590db5cd8  libSDL2-2.0.so.0
    adfaba3ed88c5e5acd521384f047bc369f52578f01d0b6b39fc7591e94e93944  libEGL.so
    db7ad1f59cbac5a23aad9dd7eba87322b5921c486097c1140b24f2d82af28892  libjson-c.so.5
    5822e549fae04bdf4ea2af7efb4f39f15c398066d3107fa576467b0d88947145  fallback/libGLESv2.so
    b92c2058527c31757298112a563a44c987b48dc35c64ab2ba41ac9c5183057c9  fallback/libshmvar.so

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
