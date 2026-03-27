import tkinter as tk
from tkinter import ttk
import sys

from .network import URL, lex

WIDTH, HEIGHT = 800, 600


class Browser:
    def __init__(self):
        self.window = tk.Tk()
        self.canvas = tk.Canvas(
            self.window,
            width=WIDTH,
            height=HEIGHT
        )
        self.canvas.pack()

    def load(self, url):
        body = url.request()
        text = lex(body)
        for c in text:
            self.canvas.create_text(100, 100, text=c)


def launch(url=None):
    browser = Browser()
    if url is not None:
        browser.load(URL(url))
    browser.window.mainloop()

if __name__ == "__main__":
    launch(sys.argv[1] if len(sys.argv) > 1 else None)
