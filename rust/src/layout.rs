use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::{HEIGHT, HSTEP, SCROLLBAR_WIDTH, VSTEP, WIDTH};
use crate::emoji::has_emoji_asset;
use crate::parser::{extract_text, Element, Node, Text};

pub type DisplayItem = (f32, f32, String, bool, bool, f32, String);
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
    pub children: Vec<BlockLayout>,
    width: f32,
    rtl: bool,
    x: f32,
    y: f32,
    height: f32,
}

pub struct BlockLayout {
    node: Node,
    pub children: Vec<BlockLayout>,
    display_list: Vec<DisplayItem>,
    width: f32,
    rtl: bool,
    x: f32,
    y: f32,
    height: f32,
    cursor_x: f32,
    cursor_y: f32,
    line: Vec<(f32, String, bool, bool, f32, String)>,
}

pub enum DrawCommand {
    Text(DrawText),
    Emoji(DrawEmoji),
    Rect(DrawRect),
}

pub struct DrawText {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    text: String,
    bold: bool,
    italic: bool,
    size: f32,
    color: String,
}

pub struct DrawEmoji {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    text: String,
    bold: bool,
    italic: bool,
    size: f32,
    color: String,
}

pub struct DrawRect {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    right: f32,
    color: egui::Color32,
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
        }
    }

    pub fn layout(&mut self, ctx: &egui::Context, font_cache: &mut FontCache) {
        let mut child = BlockLayout::new(&self.node, self.width, self.rtl);
        self.width = WIDTH - 2.0 * HSTEP;
        self.x = HSTEP;
        self.y = VSTEP;
        child.layout(ctx, font_cache, self.x, self.y, self.width, 0.0, None);
        self.height = child.height;
        self.children.clear();
        self.children.push(child);
        let _ = HEIGHT;
    }

    pub fn paint(&self) -> Vec<DrawCommand> {
        Vec::new()
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
            self.line.clear();
            let node = self.node.clone();
            let initial_style = node_style(&node).clone();
            self.recurse(&node, &initial_style, ctx, font_cache);
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

    fn close_tag(&mut self, tag: &str, ctx: &egui::Context, font_cache: &mut FontCache) {
        match tag {
            "div" => self.newline(ctx, font_cache),
            "p" => {
                self.newline(ctx, font_cache);
                self.cursor_y += VSTEP;
            }
            _ => {}
        }
    }

    fn recurse(
        &mut self,
        tree: &Node,
        inherited_style: &HashMap<String, String>,
        ctx: &egui::Context,
        font_cache: &mut FontCache,
    ) {
        match tree {
            Node::Text(Text { .. }) => {
                for word in extract_text(tree).split_whitespace() {
                    self.word(inherited_style, &word, ctx, font_cache);
                }
            }
            Node::Element(element) => {
                let normalized = element.tag.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "br" | "br/") {
                    self.newline(ctx, font_cache);
                    return;
                }
                let current_style = node_style(tree);
                for child in &element.children {
                    self.recurse(child, current_style, ctx, font_cache);
                }
                self.close_tag(&element.tag, ctx, font_cache);
            }
        }
    }

    fn word(
        &mut self,
        style: &HashMap<String, String>,
        word: &str,
        ctx: &egui::Context,
        font_cache: &mut FontCache,
    ) {
        let color = style
            .get("color")
            .cloned()
            .unwrap_or_else(|| "black".to_string());
        let bold = style
            .get("font-weight")
            .is_some_and(|weight| weight == "bold");
        let italic = style
            .get("font-style")
            .is_some_and(|style| style == "italic");
        let size = style
            .get("font-size")
            .and_then(|size| size.strip_suffix("px"))
            .and_then(|size| size.parse::<f32>().ok())
            .unwrap_or(BASE_FONT_SIZE);
        let w = font_cache.measure_text(ctx, word, bold, italic, size);

        if self.cursor_x + w > self.width - SCROLLBAR_WIDTH {
            self.flush(ctx, font_cache);
        }

        self.line
            .push((self.cursor_x, word.to_string(), bold, italic, size, color));
        self.cursor_x += font_cache.measure_text(ctx, &format!("{word} "), bold, italic, size);
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
            .map(|(_, _, bold, italic, size, _)| {
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

        for (rel_x, word, bold, italic, size, color) in &self.line {
            let x = self.x + rel_x + shift;
            let y = self.y + baseline - font_cache.measure_metrics(ctx, *bold, *italic, *size).ascent;
            self.display_list
                .push((x, y, word.clone(), *bold, *italic, *size, color.clone()));
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
        let (last_x, last_word, last_bold, last_italic, last_size, _) = self.line.last().unwrap();
        last_x + font_cache.measure_text(ctx, last_word, *last_bold, *last_italic, *last_size)
            - HSTEP
    }

    pub fn paint(&self) -> Vec<DrawCommand> {
        let mut commands = Vec::new();

        let bgcolor = node_style(&self.node)
            .get("background-color")
            .cloned()
            .unwrap_or_else(|| "transparent".to_string());
        if bgcolor != "transparent" {
            let x2 = self.x + self.width;
            let y2 = self.y + self.height;
            commands.push(DrawCommand::Rect(DrawRect {
                top: self.y,
                left: self.x,
                bottom: y2,
                right: x2,
                color: parse_color(&bgcolor),
            }));
        }

        if self.layout_mode() == "inline" {
            for (x, y, word, bold, italic, size, color) in &self.display_list {
                let top = *y;
                let bottom = *y + *size * 1.25;
                if has_emoji_asset(word) {
                    commands.push(DrawCommand::Emoji(DrawEmoji {
                        top,
                        left: *x,
                        bottom,
                        text: word.clone(),
                        bold: *bold,
                        italic: *italic,
                        size: *size,
                        color: color.clone(),
                    }));
                } else {
                    commands.push(DrawCommand::Text(DrawText {
                        top,
                        left: *x,
                        bottom,
                        text: word.clone(),
                        bold: *bold,
                        italic: *italic,
                        size: *size,
                        color: color.clone(),
                    }));
                }
            }
        }

        commands
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

impl DrawCommand {
    pub fn top(&self) -> f32 {
        match self {
            Self::Text(cmd) => cmd.top,
            Self::Emoji(cmd) => cmd.top,
            Self::Rect(cmd) => cmd.top,
        }
    }

    pub fn bottom(&self) -> f32 {
        match self {
            Self::Text(cmd) => cmd.bottom,
            Self::Emoji(cmd) => cmd.bottom,
            Self::Rect(cmd) => cmd.bottom,
        }
    }

    pub fn execute(
        &self,
        scroll: f32,
        painter: &egui::Painter,
        ctx: &egui::Context,
        color: egui::Color32,
        emoji_cache: &mut crate::emoji::EmojiCache,
        font_cache: &mut FontCache,
    ) {
        match self {
            Self::Text(cmd) => cmd.execute(scroll, painter, ctx, color, font_cache),
            Self::Emoji(cmd) => cmd.execute(scroll, painter, ctx, color, emoji_cache, font_cache),
            Self::Rect(cmd) => cmd.execute(scroll, painter),
        }
    }
}

impl DrawText {
    fn execute(
        &self,
        scroll: f32,
        painter: &egui::Painter,
        ctx: &egui::Context,
        _color: egui::Color32,
        font_cache: &mut FontCache,
    ) {
        let galley = font_cache.layout_word(ctx, &self.text, self.bold, self.italic, self.size);
        let pos = egui::pos2(self.left, self.top - scroll);
        painter.galley(pos, galley, parse_color(&self.color));
    }
}

impl DrawEmoji {
    fn execute(
        &self,
        scroll: f32,
        painter: &egui::Painter,
        ctx: &egui::Context,
        _color: egui::Color32,
        emoji_cache: &mut crate::emoji::EmojiCache,
        font_cache: &mut FontCache,
    ) {
        if let Some(texture) = emoji_cache.load(ctx, &self.text) {
            let rect = egui::Rect::from_min_size(
                egui::pos2(self.left, self.top - scroll),
                egui::vec2(crate::constants::EMOJI_SIZE as f32, crate::constants::EMOJI_SIZE as f32),
            );
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            let galley = font_cache.layout_word(ctx, &self.text, self.bold, self.italic, self.size);
            let pos = egui::pos2(self.left, self.top - scroll);
            painter.galley(pos, galley, parse_color(&self.color));
        }
    }
}

impl DrawRect {
    fn execute(&self, scroll: f32, painter: &egui::Painter) {
        let rect = egui::Rect::from_min_max(
            egui::pos2(self.left, self.top - scroll),
            egui::pos2(self.right, self.bottom - scroll),
        );
        painter.rect_filled(rect, 0.0, self.color);
    }
}

fn node_style(node: &Node) -> &HashMap<String, String> {
    match node {
        Node::Text(text) => &text.style,
        Node::Element(element) => &element.style,
    }
}

fn parse_color(color: &str) -> egui::Color32 {
    match color {
        "black" => egui::Color32::BLACK,
        "white" => egui::Color32::WHITE,
        "blue" => egui::Color32::BLUE,
        "red" => egui::Color32::RED,
        "green" => egui::Color32::GREEN,
        "gray" | "grey" => egui::Color32::GRAY,
        "lightblue" => egui::Color32::from_rgb(173, 216, 230),
        _ if color.starts_with('#') && color.len() == 4 => {
            let red = &color[1..2];
            let green = &color[2..3];
            let blue = &color[3..4];
            let red = u8::from_str_radix(&format!("{red}{red}"), 16).unwrap_or(0);
            let green = u8::from_str_radix(&format!("{green}{green}"), 16).unwrap_or(0);
            let blue = u8::from_str_radix(&format!("{blue}{blue}"), 16).unwrap_or(0);
            egui::Color32::from_rgb(red, green, blue)
        }
        _ if color.starts_with('#') && color.len() == 7 => {
            let red = u8::from_str_radix(&color[1..3], 16).unwrap_or(0);
            let green = u8::from_str_radix(&color[3..5], 16).unwrap_or(0);
            let blue = u8::from_str_radix(&color[5..7], 16).unwrap_or(0);
            egui::Color32::from_rgb(red, green, blue)
        }
        _ => egui::Color32::BLACK,
    }
}
