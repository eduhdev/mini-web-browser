from .network import URL, fetch, lex


def launch_gui(*args, **kwargs):
    from .graphics import launch
    return launch(*args, **kwargs)


__all__ = ["URL", "fetch", "launch_gui", "lex"]
