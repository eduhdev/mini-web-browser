from .constants import HSTEP, SCROLLBAR_WIDTH, VSTEP
from .fonts import get_font
from .network import Text, extract_text


class Layout:
    def __init__(self, tree, width, rtl=False, font_getter=None):
        if font_getter is None:
            font_getter = get_font
        self.display_list = []
        self.width = width
        self.rtl = rtl
        self.get_font = font_getter
        self.cursor_x = HSTEP
        self.cursor_y = VSTEP
        self.weight = "normal"
        self.style = "roman"
        self.size = 12
        self.line = []

        self.recurse(tree)
        self.flush()
    
    def open_tag(self, tag):
        if tag == "i":
            self.style = "italic"
        elif tag == "b":
            self.weight = "bold"
        elif tag == "small":
            self.size -= 2
        elif tag == "big":
            self.size += 4

    def close_tag(self, tag):
        if tag == "i":
            self.style = "roman"
        elif tag == "b":
            self.weight = "normal"
        elif tag == "small":
            self.size += 2
        elif tag == "big":
            self.size -= 4
        elif tag == "div":
            self.newline()
        elif tag == "p":
            self.newline()
            self.cursor_y += VSTEP

    def recurse(self, tree):
        if isinstance(tree, Text):
            for word in extract_text([tree]).split():
                self.word(word)
        elif tree.tag.strip().casefold() in ["br", "br/", "/div"]:
            self.newline()
        else:
            self.open_tag(tree.tag)
            for child in tree.children:
                self.recurse(child)
            self.close_tag(tree.tag)

    def word(self, word):
        font = self.get_font(self.size, self.weight, self.style)
        w = font.measure(word)

        if self.cursor_x + w > self.width - HSTEP - SCROLLBAR_WIDTH:
            self.flush()

        self.line.append((self.cursor_x, word, font))
        self.cursor_x += font.measure(word + " ")

    def newline(self):
        self.flush()

    def flush(self):
        if not self.line:
            return

        metrics = [font.metrics() for x, word, font in self.line]
        max_ascent = max([metric["ascent"] for metric in metrics])
        baseline = self.cursor_y + 1.25 * max_ascent
        line_width = self.measure_line()
        if self.rtl:
            shift = max(HSTEP, self.width - HSTEP - SCROLLBAR_WIDTH - line_width) - HSTEP
        else:
            shift = 0

        for x, word, font in self.line:
            y = baseline - font.metrics("ascent")
            self.display_list.append((x + shift, y, word, font))

        max_descent = max([metric["descent"] for metric in metrics])
        self.cursor_y = baseline + 1.25 * max_descent
        self.cursor_x = HSTEP
        self.line = []

    def measure_line(self):
        last_x, last_word, last_font = self.line[-1]
        return last_x + last_font.measure(last_word) - HSTEP
