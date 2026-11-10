"""Rebuild suits.ttf from the web's suits.woff2 (jdh8.github.io/fonts).

    uv run --with fonttools --with brotli assets/suits.py suits.woff2 assets/suits.ttf

GPUI's Linux text system drops any font whose cmap lacks 'm', so the four-glyph
subset never loads as-is.  Alias 'm' to the spade: the client only ever sets
suit symbols in this face, so the alias is never drawn.
"""
import sys

from fontTools.ttLib import TTFont

font = TTFont(sys.argv[1])
for table in font["cmap"].tables:
    if table.isUnicode():
        table.cmap[ord("m")] = table.cmap[0x2660]
font.flavor = None
font.save(sys.argv[2])
