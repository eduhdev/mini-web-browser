from .constants import HSTEP, SCROLLBAR_WIDTH, VSTEP, WIDTH
from .fonts import get_font
from .parser import Text, extract_text, Element

BLOCK_ELEMENTS = [
    "html", "body", "article", "section", "nav", "aside",
    "h1", "h2", "h3", "h4", "h5", "h6", "hgroup", "header",
    "footer", "address", "p", "hr", "pre", "blockquote",
    "ol", "ul", "menu", "li", "dl", "dt", "dd", "figure",
    "figcaption", "main", "div", "table", "form", "fieldset",
    "legend", "details", "summary"
]

class DocumentLayout:
    def __init__(self, node, width, rtl=False, font_getter=None):
        self.node = node
        self.parent = None
        self.children = []
        self.width = width
        self.rtl = rtl
        self.font_getter = font_getter
        self.x = None
        self.y = None
        self.width = None
        self.height = None

    def layout(self):
        child = BlockLayout(self.node, self, None, self.width, self.rtl, self.font_getter)
        self.children.append(child)
        self.width = WIDTH - 2*HSTEP
        self.x = HSTEP
        self.y = VSTEP
        child.layout()
        self.height = child.height

    def paint(self):
        return []

class BlockLayout:
    def __init__(self, node, parent, previous, width, rtl=False, font_getter=None):
        self.node = node
        self.parent = parent
        self.previous = previous
        self.children = []
        self.display_list = []
        self.width = width
        self.rtl = rtl
        self.get_font = get_font if font_getter is None else font_getter
        self.x = None
        self.y = None
        self.width = None
        self.height = None
    
    def paint(self):
        return self.display_list

    def layout(self):
        if self.previous:
            self.y = self.previous.y + self.previous.height
        else:
            self.y = self.parent.y
        self.x = self.parent.x
        self.width = self.parent.width
        mode = self.layout_mode()
        if mode == "block":
            previous = None
            for child in self.node.children:
                next = BlockLayout(child, self, previous, width=self.width, rtl=self.rtl, font_getter=self.get_font)
                self.children.append(next)
                previous = next
        else:
            self.cursor_x = 0
            self.cursor_y = 0
            self.weight = "normal"
            self.style = "roman"
            self.size = 12

            self.line = []
            self.recurse(self.node)
            self.flush()
            self.height = self.cursor_y

        for child in self.children:
            child.layout()
            self.display_list.extend(child.display_list)
        if mode == "block":
            self.height = sum([child.height for child in self.children])
    
    def layout_intermediate(self):
        previous = None
        for child in self.node.children:
            next = BlockLayout(child, self, previous)
            self.children.append(next)
            previous = next
    
    def layout_mode(self):
        if isinstance(self.node, Text):
            return "inline"
        elif any([isinstance(child, Element) and child.tag in BLOCK_ELEMENTS
            for child in self.node.children]):
                return "block"
        elif self.node.children:
            return "inline"
        else:
            return "block"
    
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

        if self.cursor_x + w > self.width - SCROLLBAR_WIDTH:
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

        for rel_x, word, font in self.line:
            x = self.x + rel_x + shift
            y = self.y + baseline - font.metrics("ascent")
            self.display_list.append((x, y, word, font))

        max_descent = max([metric["descent"] for metric in metrics])
        self.cursor_y = baseline + 1.25 * max_descent
        self.cursor_x = HSTEP
        self.line = []

    def measure_line(self):
        last_x, last_word, last_font = self.line[-1]
        return last_x + last_font.measure(last_word) - HSTEP
