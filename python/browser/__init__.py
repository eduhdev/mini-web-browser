from .network import URL, fetch
from .parser import HTMLParser


def launch_gui(*args, **kwargs):
    from .graphics import launch
    return launch(*args, **kwargs)


__all__ = ["URL", "fetch", "launch_gui", "HTMLParser"]
