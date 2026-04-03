import tkinter as tk
import argparse
import signal
from .constants import HEIGHT, SCROLL_STEP, SCROLLBAR_WIDTH, VSTEP, WIDTH
from .emoji import EmojiCache
from .fonts import get_font
from .layout import DocumentLayout
from .network import DEFAULT_FILE, URL
from .parser import HTMLParser, print_tree

def paint_tree(layout_object, display_list):
    display_list.extend(layout_object.paint())

    for child in layout_object.children:
        paint_tree(child, display_list)

class Browser:
    def __init__(self, rtl=False):
        self.window = tk.Tk()
        self.width = WIDTH
        self.height = HEIGHT
        self.nodes = None
        self.display_list = []
        self.emoji_cache = EmojiCache()
        self.rtl = rtl
        self.canvas = tk.Canvas(
            self.window,
            width=WIDTH,
            height=HEIGHT,
            highlightthickness=0
        )
        self.canvas.pack(fill="both", expand=True)
        self.scroll = 0
        self.window.bind("<Down>", self.scrolldown)
        self.window.bind("<Up>", self.scrolltop)
        self.window.bind("<Configure>", self.resize)
        self.canvas.bind("<Enter>", lambda e: self.canvas.focus_set())
        self.window.bind_all("<MouseWheel>", self.scrollmouse)
        self.window.bind_all("<TouchpadScroll>", self.scrolltouchpad)
    
    def scrolldown(self, e):
        max_y = max(self.document.height + 2*VSTEP - HEIGHT, 0)
        self.scroll = min(self.scroll + SCROLL_STEP, max_y)
        self.draw()

    def scrolltop(self, e):
        self.scrollby(-SCROLL_STEP)

    def scrollmouse(self, e):
        if e.delta == 0:
            return
        self.scrollby(-e.delta * 4)

    def scrolltouchpad(self, e):
        delta_y = e.delta & 0xffff
        if delta_y >= 0x8000:
            delta_y -= 0x10000
        if delta_y == 0:
            return
        self.scrollby(-delta_y)

    def scrollby(self, amount):
        max_scroll = self.max_scroll()
        new_scroll = min(max(self.scroll + amount, 0), max_scroll)
        if new_scroll == self.scroll:
            return
        self.scroll = new_scroll
        self.draw()
        
    def draw(self):
        self.canvas.delete("all")
        for cmd in self.display_list:
            if cmd.top > self.scroll + HEIGHT: continue
            if cmd.bottom < self.scroll: continue
            cmd.execute(self.scroll, self.canvas)
        self.draw_scrollbar()

    def load(self, url):
        body = url.request()
        self.body = body
        self.nodes = HTMLParser(body).parse()
        print_tree(self.nodes)
        self.document = DocumentLayout(self.nodes, self.width, self.rtl, get_font)
        self.document.layout()
        self.scroll = 0
        self.display_list = []
        paint_tree(self.document, self.display_list)
        self.draw()

    def resize(self, e):
        self.width = e.width
        self.height = e.height
        if not self.nodes:
            return
        self.document = DocumentLayout(self.nodes, self.width, self.rtl, get_font)
        self.document.layout()
        self.scroll = min(self.scroll, self.max_scroll())
        self.display_list = []
        paint_tree(self.document, self.display_list)
        self.draw()

    def document_height(self):
        if not self.display_list:
            return self.height
        return self.display_list[-1].bottom + VSTEP

    def max_scroll(self):
        return max(self.document_height() - self.height, 0)

    def draw_scrollbar(self):
        document_height = self.document_height()
        if document_height <= self.height:
            return

        top = self.scroll / document_height * self.height
        bottom = (self.scroll + self.height) / document_height * self.height
        self.canvas.create_rectangle(
            self.width - SCROLLBAR_WIDTH,
            top,
            self.width,
            bottom,
            fill="light blue",
            outline="light blue"
        )

def launch(url=None, rtl=False):
    browser = Browser(rtl=rtl)
    if url is None:
        url = "file://" + str(DEFAULT_FILE)
    browser.load(URL(url))
    previous_sigint_handler = signal.getsignal(signal.SIGINT)
    previous_sigtstp_handler = (
        signal.getsignal(signal.SIGTSTP)
        if hasattr(signal, "SIGTSTP")
        else None
    )

    def handle_stop(signum, frame):
        browser.window.after(0, browser.window.destroy)

    signal.signal(signal.SIGINT, handle_stop)
    if hasattr(signal, "SIGTSTP"):
        signal.signal(signal.SIGTSTP, handle_stop)
    browser.window.mainloop()
    signal.signal(signal.SIGINT, previous_sigint_handler)
    if previous_sigtstp_handler is not None:
        signal.signal(signal.SIGTSTP, previous_sigtstp_handler)

def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("url", nargs="?")
    parser.add_argument("--rtl", action="store_true")
    args = parser.parse_args(argv)
    launch(args.url, rtl=args.rtl)

if __name__ == "__main__":
    main()
