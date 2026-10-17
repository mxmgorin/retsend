#!/bin/sh
# Prepares a theme's font for embedding: subsets it to the scripts the UI
# shows, then sets its line metrics to those of egui's default face, so rows
# laid out for that face keep their spacing. Needs fontTools (pyftsubset).
#
#   tools/theme-font.sh <source.ttf> resources/fonts/<name>.ttf
set -eu
[ $# -eq 2 ] || { echo "usage: $0 <source.ttf> <out.ttf>" >&2; exit 1; }

# Latin, Greek, Cyrillic, punctuation, currency, letterlike symbols, arrows,
# math operators, geometric shapes.
UNICODES="U+0020-007E,U+00A0-024F,U+0370-03FF,U+0400-04FF,U+2000-206F,U+20A0-20CF,U+2100-214F,U+2190-21FF,U+2200-22FF,U+25A0-25FF"

pyftsubset "$1" --unicodes="$UNICODES" --layout-features='*' --output-file="$2"

python3 - "$2" <<'EOF'
import sys
from fontTools.ttLib import TTFont

# egui's default face (Ubuntu-Light): ascender, descender, line gap, per em.
UBUNTU_EM = 1000
UBUNTU_ASCENT, UBUNTU_DESCENT, UBUNTU_GAP = 932, -189, 28

font = TTFont(sys.argv[1])
k = font["head"].unitsPerEm / UBUNTU_EM
ascent, descent, gap = (round(v * k) for v in (UBUNTU_ASCENT, UBUNTU_DESCENT, UBUNTU_GAP))
hhea, os2 = font["hhea"], font["OS/2"]
hhea.ascent, hhea.descent, hhea.lineGap = ascent, descent, gap
os2.sTypoAscender, os2.sTypoDescender, os2.sTypoLineGap = ascent, descent, gap
font.save(sys.argv[1])
EOF
