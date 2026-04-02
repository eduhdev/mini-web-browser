class Text:
    def __init__(self, text, parent):
        self.text = text
        self.children = []
        self.parent = parent

    def __repr__(self):
        return repr(self.text)


class Element:
    def __init__(self, tag, attributes, parent):
        self.tag = tag
        self.children = []
        self.parent = parent
        self.attributes = attributes

    def __repr__(self):
        return "<" + self.tag + ">"


class HTMLParser:
    SELF_CLOSING_TAGS = [
        "area", "base", "br", "col", "embed", "hr", "img", "input",
        "link", "meta", "param", "source", "track", "wbr",
    ]

    HEAD_TAGS = [
        "base", "basefont", "bgsound", "noscript",
        "link", "meta", "title", "style", "script",
    ]

    def __init__(self, body):
        self.body = body
        self.unfinished = []

    def parse(self):
        text = ""
        in_tag = False
        for c in self.body:
            if c == "<":
                in_tag = True
                if text:
                    self.add_text(text)
                text = ""
            elif c == ">":
                in_tag = False
                self.add_tag(text)
                text = ""
            else:
                text += c
        if not in_tag and text:
            self.add_text(text)
        return self.finish()

    def add_text(self, text):
        if text.isspace():
            return
        self.implicit_tags(None)
        parent = self.unfinished[-1]
        node = Text(text, parent)
        parent.children.append(node)

    def get_attributes(self, text):
        parts = text.split()
        tag = parts[0].casefold()
        attributes = {}
        for attrpair in parts[1:]:
            if "=" in attrpair:
                key, value = attrpair.split("=", 1)
                attributes[key.casefold()] = value
                if len(value) > 2 and value[0] in ["'", "\""]:
                    value = value[1:-1]
            else:
                attributes[attrpair.casefold()] = ""
        return tag, attributes

    def add_tag(self, tag):
        tag, attributes = self.get_attributes(tag)
        if tag.startswith("!"):
            return
        self.implicit_tags(tag)
        if tag.startswith("/"):
            if len(self.unfinished) == 1:
                return
            node = self.unfinished.pop()
            parent = self.unfinished[-1]
            parent.children.append(node)
        elif tag in self.SELF_CLOSING_TAGS:
            parent = self.unfinished[-1]
            node = Element(tag, attributes, parent)
            parent.children.append(node)
        else:
            parent = self.unfinished[-1] if self.unfinished else None
            node = Element(tag, attributes, parent)
            self.unfinished.append(node)
    
    def implicit_tags(self, tag):
        while True:
            open_tags = [node.tag for node in self.unfinished]
            if open_tags == [] and tag != "html":
                self.add_tag("html")
            elif open_tags == ["html"] and tag not in ["head", "body", "/html"]:
                if tag in self.HEAD_TAGS:
                    self.add_tag("head")
                else:
                    self.add_tag("body")
            elif open_tags == ["html", "head"] and tag not in ["/head"] + self.HEAD_TAGS:
                self.add_tag("/head")
            else:
                break

    def finish(self):
        if not self.unfinished:
            self.implicit_tags(None)
        while len(self.unfinished) > 1:
            node = self.unfinished.pop()
            parent = self.unfinished[-1]
            parent.children.append(node)
        return self.unfinished.pop()


def print_tree(node):
    print(tree_to_html(node))


def tree_to_html(node):
    if isinstance(node, Text):
        return node.text

    attributes = "".join(
        f' {key}="{value}"' if value else f" {key}"
        for key, value in node.attributes.items()
    )
    if node.tag in HTMLParser.SELF_CLOSING_TAGS:
        return f"<{node.tag}{attributes}>"

    children = "".join(tree_to_html(child) for child in node.children)
    return f"<{node.tag}{attributes}>{children}</{node.tag}>"


def extract_text(tokens):
    text = ""
    entity = ""
    in_entity = False
    in_whitespace = False

    def visit(node):
        nonlocal text, entity, in_entity, in_whitespace

        if isinstance(node, Element):
            normalized_tag = node.tag.strip().casefold()
            if normalized_tag in ["br", "br/"]:
                text = text.rstrip(" ")
                text += "\n"
                in_whitespace = False
                return

            for child in node.children:
                visit(child)

            if normalized_tag in ["div", "p"]:
                text = text.rstrip(" ")
                text += "\n"
                in_whitespace = False
            return

        for c in node.text:
            if in_entity:
                entity += c
                if entity == "&lt;":
                    text += "<"
                    entity = ""
                    in_entity = False
                    in_whitespace = False
                elif entity == "&gt;":
                    text += ">"
                    entity = ""
                    in_entity = False
                    in_whitespace = False
                elif c == ";":
                    text += entity
                    entity = ""
                    in_entity = False
                    in_whitespace = False
                continue

            if c == "&":
                entity = c
                in_entity = True
            elif c.isspace():
                if text and not in_whitespace:
                    text += " "
                in_whitespace = True
            else:
                text += c
                in_whitespace = False

    if isinstance(tokens, list):
        for token in tokens:
            visit(token)
    else:
        visit(tokens)

    return text.strip()
