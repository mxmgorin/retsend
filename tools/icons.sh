#!/bin/sh
# Renders the packaged icons from the SVG masters in resources/icon, one per
# launcher style: muOS draws a 24x24 single-colour glyph its themes recolour,
# spruceOS a flat gold 70x70 silhouette, Android an adaptive icon whose
# foreground sits in the 66dp safe zone of a 108dp canvas. Needs rsvg-convert
# and ImageMagick 7.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
glyph="$root/resources/icon/retsend-glyph.svg"
small="$root/resources/icon/retsend-glyph-small.svg"
mark="$root/resources/icon/retsend-mark.svg"

# The glyph masters are teal; a launcher's own colour stands in for it.
tinted() { sed "s/#3fb8a0/$2/g" "$1"; }

# muOS: white, drawn unscaled at the size themes cut their glyphs to.
tinted "$small" '#ffffff' | rsvg-convert -w 24 -h 24 -o "$root/packaging/muos/glyph/retsend.png"

# spruceOS: the SPRUCE theme's gold, inset like its own app icons.
tinted "$glyph" '#d7b45f' | rsvg-convert -w 52 -h 52 | magick - -background none -gravity center -extent 70x70 \
  "$root/packaging/spruce/App/Retsend/icon.png"

# Android: the foreground layer per density; the launcher masks and parallaxes it.
for density in mdpi:108 hdpi:162 xhdpi:216 xxhdpi:324 xxxhdpi:432; do
  name=${density%%:*}
  canvas=${density##*:}
  inner=$((canvas * 72 / 108))
  rsvg-convert -w "$inner" -h "$inner" "$mark" | magick - -background none -gravity center \
    -extent "${canvas}x${canvas}" "$root/android/icon/mipmap-$name/ic_launcher_foreground.png"
done
