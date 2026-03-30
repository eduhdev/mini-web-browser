use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};
use std::collections::HashMap;
use std::sync::Arc;

use crate::constants::{HSTEP, SCROLLBAR_WIDTH, VSTEP};
use crate::network::{extract_text, Token};

pub type DisplayItem = (f32, f32, String, bool, bool, f32);
const REGULAR_FAMILY: &str = "browser-regular";
const BOLD_FAMILY: &str = "browser-bold";
const ITALIC_FAMILY: &str = "browser-italic";
const BOLD_ITALIC_FAMILY: &str = "browser-bold-italic";
const BASE_FONT_SIZE: f32 = 14.0;

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

pub struct Layout {
    pub display_list: Vec<DisplayItem>,
    width: f32,
    rtl: bool,
    cursor_x: f32,
    cursor_y: f32,
    weight: &'static str,
    style: &'static str,
    size: f32,
    line: Vec<(f32, String, bool, bool, f32)>,
}

impl Layout {
    pub fn new(
        tokens: &[Token],
        width: f32,
        rtl: bool,
        ctx: &egui::Context,
        font_cache: &mut FontCache,
    ) -> Self {
        let mut layout = Self {
            display_list: Vec::new(),
            width,
            rtl,
            cursor_x: HSTEP,
            cursor_y: VSTEP,
            weight: "normal",
            style: "roman",
            size: BASE_FONT_SIZE,
            line: Vec::new(),
        };

        for tok in tokens {
            layout.token(tok, ctx, font_cache);
        }

        layout.flush(ctx, font_cache);
        layout
    }

    fn token(&mut self, tok: &Token, ctx: &egui::Context, font_cache: &mut FontCache) {
        match tok {
            Token::Text(_) => {
                for word in extract_text(std::slice::from_ref(tok)).split_whitespace() {
                    self.word(word, ctx, font_cache);
                }
            }
            Token::Tag(tag) => match tag.as_str() {
                "i" => self.style = "italic",
                "/i" => self.style = "roman",
                "b" => self.weight = "bold",
                "/b" => self.weight = "normal",
                "small" => self.size -= 2.0,
                "/small" => self.size += 2.0,
                "big" => self.size += 4.0,
                "/big" => self.size -= 4.0,
                "/p" => {
                    self.newline(ctx, font_cache);
                    self.cursor_y += VSTEP;
                }
                _ => {
                    let normalized = tag.trim().to_ascii_lowercase();
                    if matches!(normalized.as_str(), "br" | "br/" | "/div") {
                        self.newline(ctx, font_cache);
                    }
                }
            },
        }
    }

    fn word(&mut self, word: &str, ctx: &egui::Context, font_cache: &mut FontCache) {
        let bold = self.weight == "bold";
        let italic = self.style == "italic";
        let w = font_cache.measure_text(ctx, word, bold, italic, self.size);

        if self.cursor_x + w > self.width - HSTEP - SCROLLBAR_WIDTH {
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
            .map(|(_, _, bold, italic, size)| font_cache.measure_metrics(ctx, *bold, *italic, *size))
            .collect();
        let max_ascent = metrics
            .iter()
            .map(|metric| metric.ascent)
            .fold(0.0, f32::max);
        let baseline = self.cursor_y + 1.25 * max_ascent;
        let shift = if self.rtl {
            (self.width - HSTEP - SCROLLBAR_WIDTH - self.measure_line(ctx, font_cache)).max(HSTEP)
                - HSTEP
        } else {
            0.0
        };

        for (x, word, bold, italic, size) in &self.line {
            let y = baseline - font_cache.measure_metrics(ctx, *bold, *italic, *size).ascent;
            self.display_list
                .push((x + shift, y, word.clone(), *bold, *italic, *size));
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
        let (last_x, last_word, last_bold, last_italic, last_size) =
            self.line.last().unwrap();
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
