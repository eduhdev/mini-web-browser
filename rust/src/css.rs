use std::collections::HashMap;

use crate::parser::Node;

const INHERITED_PROPERTIES: &[(&str, &str)] = &[
    ("font-size", "16px"),
    ("font-style", "normal"),
    ("font-weight", "normal"),
    ("color", "black"),
];

pub struct CssParser {
    s: String,
    i: usize,
}

#[derive(Clone)]
pub enum Selector {
    Tag(TagSelector),
    Descendant(DescendantSelector),
}

#[derive(Clone)]
pub struct TagSelector {
    tag: String,
    pub priority: usize,
}

#[derive(Clone)]
pub struct DescendantSelector {
    ancestor: Box<Selector>,
    descendant: Box<Selector>,
    pub priority: usize,
}

pub type Rule = (Selector, HashMap<String, String>);

impl CssParser {
    pub fn new(s: &str) -> Self {
        Self {
            s: s.to_owned(),
            i: 0,
        }
    }

    fn whitespace(&mut self) {
        while self.i < self.s.len() && self.current_char().is_some_and(char::is_whitespace) {
            self.i += self.current_char().unwrap().len_utf8();
        }
    }

    fn word(&mut self) -> Result<String, String> {
        let start = self.i;
        while self.i < self.s.len() {
            let c = self.current_char().unwrap();
            if c.is_alphanumeric() || "#-.%".contains(c) {
                self.i += c.len_utf8();
            } else {
                break;
            }
        }
        if self.i <= start {
            return Err("Parsing error".to_string());
        }
        Ok(self.s[start..self.i].to_string())
    }

    fn literal(&mut self, literal: char) -> Result<(), String> {
        if self.current_char() != Some(literal) {
            return Err("Parsing error".to_string());
        }
        self.i += literal.len_utf8();
        Ok(())
    }

    fn pair(&mut self) -> Result<(String, String), String> {
        let prop = self.word()?;
        self.whitespace();
        self.literal(':')?;
        self.whitespace();
        let val = self.word()?;
        Ok((prop.to_ascii_lowercase(), val))
    }

    fn body(&mut self) -> HashMap<String, String> {
        let mut pairs = HashMap::new();
        while self.i < self.s.len() && self.current_char() != Some('}') {
            match self.pair() {
                Ok((prop, val)) => {
                    pairs.insert(prop, val);
                    self.whitespace();
                    if self.literal(';').is_err() {
                        let why = self.ignore_until(&[';', '}']);
                        if why == Some(';') {
                            let _ = self.literal(';');
                            self.whitespace();
                        } else {
                            break;
                        }
                    } else {
                        self.whitespace();
                    }
                }
                Err(_) => {
                    let why = self.ignore_until(&[';', '}']);
                    if why == Some(';') {
                        let _ = self.literal(';');
                        self.whitespace();
                    } else {
                        break;
                    }
                }
            }
        }
        pairs
    }

    fn ignore_until(&mut self, chars: &[char]) -> Option<char> {
        while self.i < self.s.len() {
            let c = self.current_char().unwrap();
            if chars.contains(&c) {
                return Some(c);
            }
            self.i += c.len_utf8();
        }
        None
    }

    fn selector(&mut self) -> Result<Selector, String> {
        let mut out = Selector::Tag(TagSelector::new(&self.word()?.to_ascii_lowercase()));
        self.whitespace();
        while self.i < self.s.len() && self.current_char() != Some('{') {
            let tag = self.word()?;
            let descendant = Selector::Tag(TagSelector::new(&tag.to_ascii_lowercase()));
            out = Selector::Descendant(DescendantSelector::new(out, descendant));
            self.whitespace();
        }
        Ok(out)
    }

    pub fn parse(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        while self.i < self.s.len() {
            self.whitespace();
            match self.selector() {
                Ok(selector) => {
                    if self.literal('{').is_err() {
                        break;
                    }
                    self.whitespace();
                    let body = self.body();
                    if self.literal('}').is_err() {
                        break;
                    }
                    rules.push((selector, body));
                }
                Err(_) => {
                    let why = self.ignore_until(&['}']);
                    if why == Some('}') {
                        let _ = self.literal('}');
                        self.whitespace();
                    } else {
                        break;
                    }
                }
            }
        }
        rules
    }

    fn current_char(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }
}

impl Selector {
    fn matches(&self, node: &Node, ancestors: &[String]) -> bool {
        match self {
            Self::Tag(selector) => selector.matches(node),
            Self::Descendant(selector) => selector.matches(node, ancestors),
        }
    }

    fn priority(&self) -> usize {
        match self {
            Self::Tag(selector) => selector.priority,
            Self::Descendant(selector) => selector.priority,
        }
    }

    fn matches_tag(&self, tag: &str, ancestors: &[String]) -> bool {
        match self {
            Self::Tag(selector) => selector.tag == tag,
            Self::Descendant(selector) => selector.matches_tag(tag, ancestors),
        }
    }
}

impl TagSelector {
    fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_owned(),
            priority: 1,
        }
    }

    fn matches(&self, node: &Node) -> bool {
        matches!(node, Node::Element(element) if self.tag == element.tag)
    }
}

impl DescendantSelector {
    fn new(ancestor: Selector, descendant: Selector) -> Self {
        let priority = ancestor.priority() + descendant.priority();
        Self {
            ancestor: Box::new(ancestor),
            descendant: Box::new(descendant),
            priority,
        }
    }

    fn matches(&self, node: &Node, ancestors: &[String]) -> bool {
        self.descendant.matches(node, ancestors)
            && ancestors
                .iter()
                .enumerate()
                .any(|(index, tag)| self.ancestor.matches_tag(tag, &ancestors[..index]))
    }

    fn matches_tag(&self, tag: &str, ancestors: &[String]) -> bool {
        self.descendant.matches_tag(tag, ancestors)
            && ancestors
                .iter()
                .enumerate()
                .any(|(index, tag)| self.ancestor.matches_tag(tag, &ancestors[..index]))
    }
}

pub fn cascade_priority(rule: &Rule) -> usize {
    rule.0.priority()
}

pub fn style(node: &mut Node, rules: &[Rule]) {
    let defaults = inherited_defaults(None);
    style_with_parent(node, rules, &defaults, &[]);
}

fn style_with_parent(
    node: &mut Node,
    rules: &[Rule],
    parent_style: &HashMap<String, String>,
    ancestors: &[String],
) {
    {
        let style = node_style_mut(node);
        style.clear();
        for (property, default_value) in INHERITED_PROPERTIES {
            let value = parent_style
                .get(*property)
                .cloned()
                .unwrap_or_else(|| (*default_value).to_owned());
            style.insert((*property).to_owned(), value);
        }
    }

    for (selector, body) in rules {
        if !selector.matches(node, ancestors) {
            continue;
        }
        let style = node_style_mut(node);
        for (property, value) in body {
            style.insert(property.clone(), value.clone());
        }
    }

    let inline_style = match node {
        Node::Element(element) => element.attributes.get("style").cloned(),
        Node::Text(_) => None,
    };
    if let Some(inline_style) = inline_style {
        let pairs = CssParser::new(&inline_style).body();
        let style = node_style_mut(node);
        for (property, value) in pairs {
            style.insert(property, value);
        }
    }

    normalize_font_size(node, parent_style);

    let current_style = node_style(node).clone();
    let mut child_ancestors = ancestors.to_vec();
    if let Node::Element(element) = node {
        child_ancestors.push(element.tag.clone());
    }
    for child in node_children_mut(node) {
        style_with_parent(child, rules, &current_style, &child_ancestors);
    }
}

fn inherited_defaults(parent_style: Option<&HashMap<String, String>>) -> HashMap<String, String> {
    let mut defaults = HashMap::new();
    for (property, default_value) in INHERITED_PROPERTIES {
        let value = parent_style
            .and_then(|style| style.get(*property))
            .cloned()
            .unwrap_or_else(|| (*default_value).to_owned());
        defaults.insert((*property).to_owned(), value);
    }
    defaults
}

fn normalize_font_size(node: &mut Node, parent_style: &HashMap<String, String>) {
    let font_size = node_style(node)
        .get("font-size")
        .cloned()
        .unwrap_or_else(|| "16px".to_owned());
    if !font_size.ends_with('%') {
        return;
    }

    let parent_font_size = parent_style
        .get("font-size")
        .cloned()
        .unwrap_or_else(|| "16px".to_owned());
    let Ok(node_pct) = font_size.trim_end_matches('%').parse::<f32>() else {
        return;
    };
    let Ok(parent_px) = parent_font_size.trim_end_matches("px").parse::<f32>() else {
        return;
    };

    node_style_mut(node).insert(
        "font-size".to_owned(),
        format!("{}px", node_pct / 100.0 * parent_px),
    );
}

fn node_style(node: &Node) -> &HashMap<String, String> {
    match node {
        Node::Text(text) => &text.style,
        Node::Element(element) => &element.style,
    }
}

fn node_style_mut(node: &mut Node) -> &mut HashMap<String, String> {
    match node {
        Node::Text(text) => &mut text.style,
        Node::Element(element) => &mut element.style,
    }
}

fn node_children_mut(node: &mut Node) -> &mut Vec<Node> {
    match node {
        Node::Text(text) => &mut text.children,
        Node::Element(element) => &mut element.children,
    }
}
