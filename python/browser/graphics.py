import tkinter as tk
from tkinter import ttk
import signal
import sys

from .network import URL, lex

WIDTH, HEIGHT = 800, 600
HSTEP, VSTEP = 13, 18
SCROLL_STEP = 100
SCROLLBAR_WIDTH = 8

class Browser:
    def __init__(self):
        self.window = tk.Tk()
        self.width = WIDTH
        self.height = HEIGHT
        self.text = ""
        self.display_list = []
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
        for x, y, c in self.display_list:
            if y > self.scroll + self.height: continue
            if y + VSTEP < self.scroll: continue
            self.canvas.create_text(x, y - self.scroll, text=c)
        self.draw_scrollbar()

    def load(self, url):
        body = url.request()
        self.text = lex(body)
        self.display_list = layout(self.text, self.width)
        self.scroll = 0
        self.draw()

    def resize(self, e):
        self.width = e.width
        self.height = e.height
        if not self.text:
            return
        self.display_list = layout(self.text, self.width)
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

def layout(text, width):
    display_list = []
    cursor_x, cursor_y = HSTEP, VSTEP
    for c in text:
        if c == "\n":
            cursor_x = HSTEP
            cursor_y += VSTEP + VSTEP // 2
            continue

        display_list.append((cursor_x, cursor_y, c))
        cursor_x += HSTEP

        if cursor_x >= width - HSTEP:
            cursor_x = HSTEP
            cursor_y += VSTEP
        
    return display_list

def launch(url=None):
    browser = Browser()
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

if __name__ == "__main__":
    launch(sys.argv[1] if len(sys.argv) > 1 else None)
