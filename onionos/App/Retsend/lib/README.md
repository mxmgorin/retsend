# Bundled libraries

The Miyoo Mini's SDL2 is a fork carrying an `mmiyoo` video driver for the
SigmaStar display pipeline; no upstream SDL2 can put pixels on this panel. Every
SDL2 port on the platform ships its own copy and preloads it, and these files are
that copy — taken verbatim from the `Sonic Mania` port of the
[OnionOS Ports-Collection](https://github.com/OnionUI/Ports-Collection) at commit
`682d248b643ffe3fd88823b297936e188b61aad8`, built from
[XK9274/sdl2_miyoo](https://github.com/XK9274/sdl2_miyoo).

`libSDL2` hard-links `SDL2_image`, `SDL2_ttf` and `json-c` for its own overlay
feature, which retsend never uses but the loader still resolves, and `libEGL.so`
(a stub) because it is listed in `DT_NEEDED`. Everything else it needs —
`libGLESv2`, `libpng16`, `libz`, `libfreetype`, `libbz2`, the `libmi_*` SoC
libraries — is already on the device.

    47df88c28bc45f22870f312140d75afcc0f8dd372c7a09873aaec8769c019b45  libSDL2-2.0.so.0
    a1c61f7d0828860170cc512c63b5f2077e45da4ecfd267d39832f95a3473ba77  libSDL2_image-2.0.so.0
    04bb3eea141ec6a723930b0064fb593695df8e7f776b177cb3300a072bd8654e  libSDL2_ttf-2.0.so.0
    db7ad1f59cbac5a23aad9dd7eba87322b5921c486097c1140b24f2d82af28892  libjson-c.so.5
    adfaba3ed88c5e5acd521384f047bc369f52578f01d0b6b39fc7591e94e93944  libEGL.so

## Licences

`libSDL2-2.0.so.0`, `libSDL2_image-2.0.so.0` and `libSDL2_ttf-2.0.so.0` are
Simple DirectMedia Layer and its satellites, under the zlib licence:

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

These are altered versions: the `mmiyoo` driver is a third-party addition, not
part of upstream SDL. See the fork linked above for the changes.

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

`libEGL.so` is a 7 KB stub shipped in the same port bundle; its licence is not
stated there, and it exists only to satisfy `DT_NEEDED`. Replace it with the
device's own EGL, or drop it, if that provenance is not good enough for a
redistribution you intend.
