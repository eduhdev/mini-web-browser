import base64

import cairosvg
import tkinter as tk

from .constants import EMOJI_DIR, VSTEP


def emoji_path_for(token):
    if not token or token == "\n":
        return None
    if len(token) > 16:
        return None
    if any(ord(char) < 0x80 and (char.isalnum() or char in "{}:;,#.-_/()[]'\" ") for char in token):
        return None

    codepoints = "-".join(f"{ord(char):X}" for char in token)
    emoji_path = EMOJI_DIR / f"{codepoints}.svg"
    if emoji_path.exists():
        return emoji_path
    return None


class EmojiCache:
    def __init__(self):
        self.cache = {}

    def load(self, token):
        if token in self.cache:
            return self.cache[token]

        emoji_path = emoji_path_for(token)
        if emoji_path is None:
            self.cache[token] = None
            return None

        png = cairosvg.svg2png(
            url=str(emoji_path),
            output_width=VSTEP,
            output_height=VSTEP,
        )
        image = tk.PhotoImage(data=base64.b64encode(png).decode("ascii"))
        self.cache[token] = image
        return image
