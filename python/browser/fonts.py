import tkinter as tk
import tkinter.font

from .constants import FONT_FAMILY

FONTS = {}


def get_font(size, weight, style):
    key = (size, weight, style)
    if key not in FONTS:
        font = tkinter.font.Font(
            family=FONT_FAMILY,
            size=size,
            weight=weight,
            slant=style,
        )
        label = tk.Label(font=font)
        FONTS[key] = (font, label)
    return FONTS[key][0]
