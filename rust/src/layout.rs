use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::{HEIGHT, HSTEP, SCROLLBAR_WIDTH, VSTEP, WIDTH};
use crate::parser::{extract_text, Element, Node, Text};

pub type DisplayItem = (f32, f32, String, bool, bool, f32);
const REGULAR_FAMILY: &str = "browser-regular";
const BOLD_FAMILY: &str = "browser-bold";
const ITALIC_FAMILY: &str = "browser-italic";
const BOLD_ITALIC_FAMILY: &str = "browser-bold-italic";
const BASE_FONT_SIZE: f32 = 14.0;

const BLOCK_ELEMENTS: &[&str] = &[
    "html", "body", "article", "section", "nav", "aside", "h1", "h2", "h3", "h4", "h5",
    "h6", "hgroup", "header", "footer", "address", "p", "hr", "pre", "blockquote", "ol",
    "ul", "menu", "li", "dl", "dt", "dd", "figure", "figcaption", "main", "div", "table",
    "form", "fieldset", "legend", "details", "summary",
];

#[derive(Clone, Eq, Hash, PartialEq)]
struct WordKey {
    text: String,
    style: StyleKey,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct StyleKey {
    size_bits: u32,
    bold: bool,
    italic: bool,
}

pub struct FontCache {
    layouts: HashMap<WordKey, Arc<egui::Galley>>,
    metrics: HashMap<StyleKey, FontMetrics>,
}

pub struct DocumentLayout {
    node: Node,
    children: Vec<BlockLayout>,
    width: f32,
    rtl: bool,
    x: f32,
    y: f32,
    height: f32,
    pub display_list: Vec<DisplayItem>,
}

pub struct BlockLayout {
    node: Node,
    children: Vec<BlockLayout>,
    display_list: Vec<DisplayItem>,
    width: f32,
    rtl: bool,
    x: f32,
    y: f32,
    height: f32,
    cursor_x: f32,
    cursor_y: f32,
    weight: &'static str,
    style: &'static str,
    size: f32,
    line: Vec<(f32, String, bool, bool, f32)>,
}

impl DocumentLayout {
    pub fn new(node: &Node, width: f32, rtl: bool) -> Self {
        Self {
            node: node.clone(),
            children: Vec::new(),
            width,
            rtl,
            x: 0.0,
            y: 0.0,
            height: 0.0,
            display_list: Vec::new(),
        }
    }

    pub fn layout(&mut self, ctx: &egui::Context, font_cache: &mut FontCache) {
        let mut child = BlockLayout::new(&self.node, self.width, self.rtl);
        self.width = WIDTH - 2.0 * HSTEP;
        self.x = HSTEP;
        self.y = VSTEP;
        child.layout(ctx, font_cache, self.x, self.y, self.width, 0.0, None);
        self.display_list = child.display_list.clone();
        self.height = child.height;
        self.children.clear();
        self.children.push(child);
        let _ = HEIGHT;
    }
}

impl BlockLayout {
    fn new(node: &Node, width: f32, rtl: bool) -> Self {
        Self {
            node: node.clone(),
            children: Vec::new(),
            display_list: Vec::new(),
            width,
            rtl,
            x: 0.0,
            y: 0.0,
            height: 0.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            weight: "normal",
            style: "roman",
            size: BASE_FONT_SIZE,
            line: Vec::new(),
        }
    }

    fn layout(
        &mut self,
        ctx: &egui::Context,
        font_cache: &mut FontCache,
        parent_x: f32,
        parent_y: f32,
        parent_width: f32,
        previous_height: f32,
        previous_y: Option<f32>,
    ) {
        self.y = previous_y.map(|y| y + previous_height).unwrap_or(parent_y);
        self.x = parent_x;
        self.width = parent_width;

        let mode = self.layout_mode();
        if mode == "block" {
            let mut previous_height = 0.0;
            let mut previous_y = None;
            let children = self.node_children().to_vec();
            for child in &children {
                let mut next = BlockLayout::new(child, self.width, self.rtl);
                next.layout(
                    ctx,
                    font_cache,
                    self.x,
                    self.y,
                    self.width,
                    previous_height,
                    previous_y,
                );
                previous_height = next.height;
                previous_y = Some(next.y);
                self.display_list.extend(next.display_list.clone());
                self.children.push(next);
            }
            self.height = self.children.iter().map(|child| child.height).sum();
        } else {
            self.cursor_x = 0.0;
            self.cursor_y = 0.0;
            self.weight = "normal";
            self.style = "roman";
            self.size = BASE_FONT_SIZE;
            self.line.clear();
            let node = self.node.clone();
            self.recurse(&node, ctx, font_cache);
            self.flush(ctx, font_cache);
            self.height = self.cursor_y;
        }
    }

    fn node_children(&self) -> &[Node] {
        match &self.node {
            Node::Text(_) => &[],
            Node::Element(element) => &element.children,
        }
    }

    fn layout_mode(&self) -> &'static str {
        match &self.node {
            Node::Text(_) => "inline",
            Node::Element(element)
                if element.children.iter().any(|child| {
                    matches!(child, Node::Element(Element { tag, .. }) if BLOCK_ELEMENTS.contains(&tag.as_str()))
                }) =>
            {
                "block"
            }
            Node::Element(element) if !element.children.is_empty() => "inline",
            Node::Element(_) => "block",
        }
    }

    fn open_tag(&mut self, tag: &str) {
        match tag {
            "i" => self.style = "italic",
            "b" => self.weight = "bold",
            "small" => self.size -= 2.0,
            "big" => self.size += 4.0,
            _ => {}
        }
    }

    fn close_tag(&mut self, tag: &str, ctx: &egui::Context, font_cache: &mut FontCache) {
        match tag {
            "i" => self.style = "roman",
            "b" => self.weight = "normal",
            "small" => self.size += 2.0,
            "big" => self.size -= 4.0,
            "div" => self.newline(ctx, font_cache),
            "p" => {
                self.newline(ctx, font_cache);
                self.cursor_y += VSTEP;
            }
            _ => {}
        }
    }

    fn recurse(&mut self, tree: &Node, ctx: &egui::Context, font_cache: &mut FontCache) {
        match tree {
            Node::Text(Text { .. }) => {
                for word in extract_text(tree).split_whitespace() {
                    self.word(word, ctx, font_cache);
                }
            }
            Node::Element(element) => {
                let normalized = element.tag.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "br" | "br/") {
                    self.newline(ctx, font_cache);
                    return;
                }
                self.open_tag(&element.tag);
                for child in &element.children {
                    self.recurse(child, ctx, font_cache);
                }
                self.close_tag(&element.tag, ctx, font_cache);
            }
        }
    }

    fn word(&mut self, word: &str, ctx: &egui::Context, font_cache: &mut FontCache) {
        let bold = self.weight == "bold";
        let italic = self.style == "italic";
        let w = font_cache.measure_text(ctx, word, bold, italic, self.size);

        if self.cursor_x + w > self.width - SCROLLBAR_WIDTH {
            self.flush(ctx, font_cache);
        }

        self.line
            .push((self.cursor_x, word.to_string(), bold, italic, self.size));
        self.cursor_x += font_cache.measure_text(ctx, &format!("{word} "), bold, italic, self.size);
    }

    fn newline(&mut self, ctx: &egui::Context, font_cache: &mut FontCache) {
        self.flush(ctx, font_cache);
    }

    fn flush(&mut self, ctx: &egui::Context, font_cache: &mut FontCache) {
        if self.line.is_empty() {
            return;
        }

        let metrics: Vec<FontMetrics> = self
            .line
            .iter()
            .map(|(_, _, bold, italic, size)| {
                font_cache.measure_metrics(ctx, *bold, *italic, *size)
            })
            .collect();
        let max_ascent = metrics
            .iter()
            .map(|metric| metric.ascent)
            .fold(0.0, f32::max);
        let baseline = self.cursor_y + 1.25 * max_ascent;
        let line_width = self.measure_line(ctx, font_cache);
        let shift = if self.rtl {
            (self.width - HSTEP - SCROLLBAR_WIDTH - line_width).max(HSTEP) - HSTEP
        } else {
            0.0
        };

        for (rel_x, word, bold, italic, size) in &self.line {
            let x = self.x + rel_x + shift;
            let y = self.y + baseline - font_cache.measure_metrics(ctx, *bold, *italic, *size).ascent;
            self.display_list
                .push((x, y, word.clone(), *bold, *italic, *size));
        }

        let max_descent = metrics
            .iter()
            .map(|metric| metric.descent)
            .fold(0.0, f32::max);
        self.cursor_y = baseline + 1.25 * max_descent;
        self.cursor_x = HSTEP;
        self.line.clear();
    }

    fn measure_line(&self, ctx: &egui::Context, font_cache: &mut FontCache) -> f32 {
        let (last_x, last_word, last_bold, last_italic, last_size) = self.line.last().unwrap();
        last_x + font_cache.measure_text(ctx, last_word, *last_bold, *last_italic, *last_size)
            - HSTEP
    }
}

#[derive(Clone, Copy)]
struct FontMetrics {
    ascent: f32,
    descent: f32,
}

impl FontCache {
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
            metrics: HashMap::new(),
        }
    }

    pub fn layout_word(
        &mut self,
        ctx: &egui::Context,
        text: &str,
        bold: bool,
        italic: bool,
        size: f32,
    ) -> Arc<egui::Galley> {
        let key = WordKey {
            text: text.to_owned(),
            style: StyleKey::new(size, bold, italic),
        };
        self.layouts
            .entry(key)
            .or_insert_with(|| build_layout_word(ctx, text, bold, italic, size))
            .clone()
    }

    fn measure_text(
        &mut self,
        ctx: &egui::Context,
        text: &str,
        bold: bool,
        italic: bool,
        size: f32,
    ) -> f32 {
        self.layout_word(ctx, text, bold, italic, size).size().x
    }

    fn measure_metrics(
        &mut self,
        ctx: &egui::Context,
        bold: bool,
        italic: bool,
        size: f32,
    ) -> FontMetrics {
        let key = StyleKey::new(size, bold, italic);
        if let Some(metrics) = self.metrics.get(&key) {
            return *metrics;
        }

        let height = self.layout_word(ctx, "Ag", bold, italic, size).size().y;
        let metrics = FontMetrics {
            ascent: height * 0.8,
            descent: height * 0.2,
        };
        self.metrics.insert(key, metrics);
        metrics
    }
}

impl StyleKey {
    fn new(size: f32, bold: bool, italic: bool) -> Self {
        Self {
            size_bits: size.to_bits(),
            bold,
            italic,
        }
    }
}

fn build_layout_word(
    ctx: &egui::Context,
    text: &str,
    bold: bool,
    italic: bool,
    size: f32,
) -> Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    let mut format = TextFormat {
        font_id: egui::FontId {
            size,
            family: font_family_for(bold, italic),
        },
        color: egui::Color32::PLACEHOLDER,
        ..Default::default()
    };
    format.italics = italic;
    job.append(text, 0.0, format);
    ctx.fonts_mut(|fonts| fonts.layout_job(job))
}

fn font_family_for(bold: bool, italic: bool) -> egui::FontFamily {
    let name = match (bold, italic) {
        (true, true) => BOLD_ITALIC_FAMILY,
        (true, false) => BOLD_FAMILY,
        (false, true) => ITALIC_FAMILY,
        (false, false) => REGULAR_FAMILY,
    };
    egui::FontFamily::Name(Arc::<str>::from(name))
}
