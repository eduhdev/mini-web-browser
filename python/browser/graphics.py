import tkinter as tk
from tkinter import ttk
import sys

from .network import URL, lex

WIDTH, HEIGHT = 800, 600
HSTEP, VSTEP = 13, 18
SCROLL_STEP = 100

class Browser:
    def __init__(self):
        self.window = tk.Tk()
        self.canvas = tk.Canvas(
            self.window,
            width=WIDTH,
            height=HEIGHT
        )
        self.canvas.pack()
        self.scroll = 0
        self.window.bind("<Down>", self.scrolldown)
        self.window.bind("<Up>", self.scrolltop)
    
    def scrolldown(self, e):
        self.scroll += SCROLL_STEP
        self.draw()

    def scrolltop(self, e):
        if self.scroll == 0:
            return
        self.scroll -= SCROLL_STEP
        self.draw()
        
    def draw(self):
        self.canvas.delete("all")
        for x, y, c in self.display_list:
            self.canvas.create_text(x, y - self.scroll, text=c)

    def load(self, url):
        body = url.request()
        text = lex(body)

        self.display_list = layout(text)
        self.draw()

def layout(text):
    display_list = []
    cursor_x, cursor_y = HSTEP, VSTEP
    for c in text:
        display_list.append((cursor_x, cursor_y, c))
        cursor_x += HSTEP

        if cursor_x >= WIDTH - HSTEP:
            cursor_x = HSTEP
            cursor_y += VSTEP
        
    return display_list

def launch(url=None):
    browser = Browser()
    if url is not None:
        browser.load(URL(url))
    browser.window.mainloop()

if __name__ == "__main__":
    launch(sys.argv[1] if len(sys.argv) > 1 else None)
