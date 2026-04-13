use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Text {
    pub text: String,
    pub children: Vec<Node>,
    pub style: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Element {
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Node>,
    pub style: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum Node {
    Text(Text),
    Element(Element),
}

pub struct HtmlParser {
    body: String,
    unfinished: Vec<Element>,
}

pub fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    let mut entity = String::new();
    let mut in_entity = false;
    let mut in_whitespace = false;

    fn visit(
        node: &Node,
        text: &mut String,
        entity: &mut String,
        in_entity: &mut bool,
        in_whitespace: &mut bool,
    ) {
        match node {
            Node::Element(element) => {
                let normalized = element.tag.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "br" | "br/") {
                    while text.ends_with(' ') {
                        text.pop();
                    }
                    text.push('\n');
                    *in_whitespace = false;
                    return;
                }

                for child in &element.children {
                    visit(child, text, entity, in_entity, in_whitespace);
                }

                if matches!(normalized.as_str(), "div" | "p") {
                    while text.ends_with(' ') {
                        text.pop();
                    }
                    text.push('\n');
                    *in_whitespace = false;
                }
            }
            Node::Text(text_node) => {
                for c in text_node.text.chars() {
                    if *in_entity {
                        entity.push(c);

                        if entity == "&lt;" {
                            text.push('<');
                            entity.clear();
                            *in_entity = false;
                            *in_whitespace = false;
                        } else if entity == "&gt;" {
                            text.push('>');
                            entity.clear();
                            *in_entity = false;
                            *in_whitespace = false;
                        } else if c == ';' {
                            text.push_str(entity);
                            entity.clear();
                            *in_entity = false;
                            *in_whitespace = false;
                        }
                        continue;
                    }

                    if c == '&' {
                        entity.push(c);
                        *in_entity = true;
                    } else if c.is_whitespace() {
                        if !text.is_empty() && !*in_whitespace {
                            text.push(' ');
                        }
                        *in_whitespace = true;
                    } else {
                        text.push(c);
                        *in_whitespace = false;
                    }
                }
            }
        }
    }

    visit(
        node,
        &mut text,
        &mut entity,
        &mut in_entity,
        &mut in_whitespace,
    );

    text.trim().to_string()
}

pub fn print_tree(node: &Node) {
    println!("{}", tree_to_html(node));
}

fn tree_to_html(node: &Node) -> String {
    match node {
        Node::Text(text) => text.text.clone(),
        Node::Element(element) => {
            let attributes = element
                .attributes
                .iter()
                .map(|(key, value)| {
                    if value.is_empty() {
                        format!(" {key}")
                    } else {
                        format!(" {key}=\"{value}\"")
                    }
                })
                .collect::<String>();

            if HtmlParser::SELF_CLOSING_TAGS.contains(&element.tag.as_str()) {
                format!("<{}{}>", element.tag, attributes)
            } else {
                let children = element.children.iter().map(tree_to_html).collect::<String>();
                if matches!(element.tag.as_str(), "html" | "head" | "body" | "div" | "p") {
                    format!(
                        "<{}{}>\n{}\n</{}>",
                        element.tag, attributes, children, element.tag
                    )
                } else {
                    format!("<{}{}>{}</{}>", element.tag, attributes, children, element.tag)
                }
            }
        }
    }
}

impl HtmlParser {
    const SELF_CLOSING_TAGS: [&'static str; 14] = [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ];
    const HEAD_TAGS: [&'static str; 9] = [
        "base",
        "basefont",
        "bgsound",
        "noscript",
        "link",
        "meta",
        "title",
        "style",
        "script",
    ];

    pub fn new(body: &str) -> Self {
        Self {
            body: body.to_owned(),
            unfinished: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Node {
        let mut text = String::new();
        let mut in_tag = false;
        let body = self.body.clone();

        for c in body.chars() {
            if c == '<' {
                in_tag = true;
                if !text.is_empty() {
                    self.add_text(&text);
                }
                text.clear();
            } else if c == '>' {
                in_tag = false;
                self.add_tag(&text);
                text.clear();
            } else {
                text.push(c);
            }
        }

        if !in_tag && !text.is_empty() {
            self.add_text(&text);
        }
        self.finish()
    }

    fn add_text(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        self.implicit_tags(None);
        let parent = self
            .unfinished
            .last_mut()
            .expect("text node without an open parent element");
        parent.children.push(Node::Text(Text {
            text: text.to_owned(),
            children: Vec::new(),
            style: HashMap::new(),
        }));
    }

    fn get_attributes(text: &str) -> (String, HashMap<String, String>) {
        let mut parts = text.split_whitespace();
        let tag = parts.next().unwrap_or("").to_ascii_lowercase();
        let mut attributes = HashMap::new();

        for attr_pair in parts {
            if let Some((key, mut value)) = attr_pair.split_once('=') {
                if value.len() > 2 && matches!(value.chars().next(), Some('"') | Some('\'')) {
                    value = &value[1..value.len() - 1];
                }
                attributes.insert(key.to_ascii_lowercase(), value.to_owned());
            } else {
                attributes.insert(attr_pair.to_ascii_lowercase(), String::new());
            }
        }

        (tag, attributes)
    }

    fn add_tag(&mut self, raw_tag: &str) {
        let (tag, attributes) = Self::get_attributes(raw_tag);
        if tag.is_empty() || tag.starts_with('!') {
            return;
        }
        self.implicit_tags(Some(tag.as_str()));

        if tag.starts_with('/') {
            if self.unfinished.len() == 1 {
                return;
            }
            let node = self.unfinished.pop().expect("missing unfinished node");
            let parent = self.unfinished.last_mut().expect("missing parent node");
            parent.children.push(Node::Element(node));
        } else if Self::SELF_CLOSING_TAGS.contains(&tag.as_str()) {
            let parent = self
                .unfinished
                .last_mut()
                .expect("self-closing tag without an open parent element");
            parent.children.push(Node::Element(Element {
                tag,
                attributes,
                children: Vec::new(),
                style: HashMap::new(),
            }));
        } else {
            self.unfinished.push(Element {
                tag,
                attributes,
                children: Vec::new(),
                style: HashMap::new(),
            });
        }
    }

    fn implicit_tags(&mut self, tag: Option<&str>) {
        loop {
            let open_tags: Vec<&str> = self.unfinished.iter().map(|node| node.tag.as_str()).collect();
            if open_tags.is_empty() && tag != Some("html") {
                self.add_tag("html");
            } else if open_tags == ["html"] && !matches!(tag, Some("head" | "body" | "/html")) {
                if let Some(tag) = tag {
                    if Self::HEAD_TAGS.contains(&tag) {
                        self.add_tag("head");
                    } else {
                        self.add_tag("body");
                    }
                } else {
                    self.add_tag("body");
                }
            } else if open_tags == ["html", "head"]
                && !matches!(tag, Some("/head"))
                && !tag.is_some_and(|tag| Self::HEAD_TAGS.contains(&tag))
            {
                self.add_tag("/head");
            } else {
                break;
            }
        }
    }

    fn finish(&mut self) -> Node {
        if self.unfinished.is_empty() {
            self.implicit_tags(None);
        }
        while self.unfinished.len() > 1 {
            let node = self.unfinished.pop().expect("missing unfinished node");
            let parent = self.unfinished.last_mut().expect("missing parent node");
            parent.children.push(Node::Element(node));
        }
        Node::Element(self.unfinished.pop().expect("parser produced no root element"))
    }
}
