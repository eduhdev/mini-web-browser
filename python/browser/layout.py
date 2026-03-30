import tkinter.font

from .constants import FONT_FAMILY, HSTEP, SCROLLBAR_WIDTH, VSTEP
from .network import Text, extract_text


class Layout:
    def __init__(self, tokens, width, rtl=False, get_font=None):
        if get_font is None:
            font_cache = {}

            def get_font(weight="normal", style="roman", size=12):
                key = (weight, style, size)
                if key not in font_cache:
                    font_cache[key] = tkinter.font.Font(
                        family=FONT_FAMILY,
                        size=size,
                        weight=weight,
                        slant=style,
                    )
                return font_cache[key]

        self.display_list = []
        self.width = width
        self.rtl = rtl
        self.get_font = get_font
        self.cursor_x = HSTEP
        self.cursor_y = VSTEP
        self.weight = "normal"
        self.style = "roman"
        self.size = 12
        self.line = []

        for tok in tokens:
            self.token(tok)

        self.flush()

    def token(self, tok):
        if isinstance(tok, Text):
            for word in extract_text([tok]).split():
                self.word(word)
        elif tok.tag == "i":
            self.style = "italic"
        elif tok.tag == "/i":
            self.style = "roman"
        elif tok.tag == "b":
            self.weight = "bold"
        elif tok.tag == "/b":
            self.weight = "normal"
        elif tok.tag == "small":
            self.size -= 2
        elif tok.tag == "/small":
            self.size += 2
        elif tok.tag == "big":
            self.size += 4
        elif tok.tag == "/big":
            self.size -= 4
        elif tok.tag.strip().casefold() in ["br", "br/", "/div"]:
            self.newline()

    def word(self, word):
        font = self.get_font(self.weight, self.style, self.size)
        w = font.measure(word)

        if self.cursor_x + w > self.width - HSTEP - SCROLLBAR_WIDTH:
            self.flush()
            self.cursor_y += self.line_height()
            self.cursor_x = HSTEP

        self.line.append((word, font))
        self.cursor_x += font.measure(word + " ")

    def newline(self):
        self.flush()
        self.cursor_y += self.line_height()
        self.cursor_x = HSTEP

    def flush(self):
        if not self.line:
            return

        line_width = self.measure_line()
        if self.rtl:
            cursor_x = max(HSTEP, self.width - HSTEP - SCROLLBAR_WIDTH - line_width)
        else:
            cursor_x = HSTEP

        for word, font in self.line:
            self.display_list.append((cursor_x, self.cursor_y, word, font))
            cursor_x += font.measure(word + " ")

        self.line = []

    def measure_line(self):
        width = 0
        for i, (word, font) in enumerate(self.line):
            width += font.measure(word)
            if i < len(self.line) - 1:
                width += font.measure(" ")
        return width

    def line_height(self):
        return int(
            self.get_font(self.weight, self.style, self.size).metrics("linespace") * 1.25
        )
