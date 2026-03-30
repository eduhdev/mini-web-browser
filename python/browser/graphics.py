import tkinter as tk
from tkinter import ttk
import tkinter.font
import argparse
import base64
import signal
import sys
from pathlib import Path

import cairosvg

from .network import Tag, Text, URL, extract_text, lex

WIDTH, HEIGHT = 800, 600
HSTEP, VSTEP = 13, 18
SCROLL_STEP = 100
SCROLLBAR_WIDTH = 8
EMOJI_DIR = Path(__file__).resolve().parents[2] / "openmoji"
FONT_FAMILY = ".AppleSystemUIFont"
FONT_SIZE = 16

class Browser:
    def __init__(self, rtl=False):
        self.window = tk.Tk()
        self.width = WIDTH
        self.height = HEIGHT
        self.tokens = []
        self.display_list = []
        self.emoji_cache = {}
        self.font_cache = {}
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
        self.scrollby(SCROLL_STEP)

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
        for x, y, token, font in self.display_list:
            if y > self.scroll + self.height: continue
            if y + VSTEP < self.scroll: continue
            emoji = self.load_emoji(token)
            if emoji is not None:
                self.canvas.create_image(x, y - self.scroll, image=emoji, anchor="nw")
            else:
                self.canvas.create_text(
                    x, y - self.scroll, text=token, anchor="nw", font=font
                )
        self.draw_scrollbar()

    def load_emoji(self, token):
        if token in self.emoji_cache:
            return self.emoji_cache[token]

        emoji_path = emoji_path_for(token)
        if emoji_path is None:
            self.emoji_cache[token] = None
            return None

        png = cairosvg.svg2png(
            url=str(emoji_path),
            output_width=VSTEP,
            output_height=VSTEP,
        )
        image = tk.PhotoImage(data=base64.b64encode(png).decode("ascii"))
        self.emoji_cache[token] = image
        return image

    def load(self, url):
        body = url.request()
        self.tokens = lex(body)
        self.display_list = layout(self.tokens, self.width, self.rtl, self.get_font)
        self.scroll = 0
        self.draw()

    def resize(self, e):
        self.width = e.width
        self.height = e.height
        if not self.tokens:
            return
        self.display_list = layout(self.tokens, self.width, self.rtl, self.get_font)
        self.scroll = min(self.scroll, self.max_scroll())
        self.draw()

    def document_height(self):
        if not self.display_list:
            return self.height
        return self.display_list[-1][1] + VSTEP

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

    def get_font(self, weight="normal", style="roman"):
        key = (weight, style)
        if key not in self.font_cache:
            self.font_cache[key] = tkinter.font.Font(
                family=FONT_FAMILY,
                size=FONT_SIZE,
                weight=weight,
                slant=style,
            )
        return self.font_cache[key]

def layout(tokens, width, rtl=False, get_font=None):
    if get_font is None:
        font_cache = {}

        def get_font(weight="normal", style="roman"):
            key = (weight, style)
            if key not in font_cache:
                font_cache[key] = tkinter.font.Font(
                    family=FONT_FAMILY,
                    size=FONT_SIZE,
                    weight=weight,
                    slant=style,
                )
            return font_cache[key]

    display_list = []
    cursor_y = VSTEP
    line = []
    cursor_x = HSTEP
    weight = "normal"
    style = "roman"

    for tok in tokens:
        if isinstance(tok, Text):
            for word in extract_text([tok]).split():
                font = get_font(weight, style)
                w = font.measure(word)
                line_height = int(font.metrics("linespace") * 1.25)

                if cursor_x + w > width - HSTEP - SCROLLBAR_WIDTH:
                    flush_line(display_list, line, cursor_y, width, rtl)
                    line = []
                    cursor_y += line_height
                    cursor_x = HSTEP

                line.append((word, font))
                cursor_x += font.measure(word + " ")
        elif tok.tag == "i":
            style = "italic"
        elif tok.tag == "/i":
            style = "roman"
        elif tok.tag == "b":
            weight = "bold"
        elif tok.tag == "/b":
            weight = "normal"
        elif tok.tag.strip().casefold() in ["br", "br/", "/div"]:
            line_height = int(get_font(weight, style).metrics("linespace") * 1.25)
            flush_line(display_list, line, cursor_y, width, rtl)
            line = []
            cursor_y += line_height
            cursor_x = HSTEP

    flush_line(display_list, line, cursor_y, width, rtl)
    return display_list

def flush_line(display_list, line, cursor_y, width, rtl):
    if not line:
        return

    line_width = measure_line(line)
    if rtl:
        cursor_x = max(HSTEP, width - HSTEP - SCROLLBAR_WIDTH - line_width)
    else:
        cursor_x = HSTEP

    for word, font in line:
        display_list.append((cursor_x, cursor_y, word, font))
        cursor_x += font.measure(word + " ")

def measure_line(line):
    width = 0
    for i, (word, font) in enumerate(line):
        width += font.measure(word)
        if i < len(line) - 1:
            width += font.measure(" ")
    return width

def emoji_path_for(token):
    if not token or token == "\n":
        return None

    codepoints = "-".join(f"{ord(char):X}" for char in token)
    emoji_path = EMOJI_DIR / f"{codepoints}.svg"
    if emoji_path.exists():
        return emoji_path
    return None

def launch(url=None, rtl=False):
    browser = Browser(rtl=rtl)
    if url is not None:
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
