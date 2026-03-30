use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};
use std::sync::Arc;

use crate::constants::{HSTEP, SCROLLBAR_WIDTH, VSTEP};
use crate::network::{extract_text, Token};

pub type DisplayItem = (f32, f32, String, bool, bool, f32);
const REGULAR_FAMILY: &str = "browser-regular";
const BOLD_FAMILY: &str = "browser-bold";
const ITALIC_FAMILY: &str = "browser-italic";
const BOLD_ITALIC_FAMILY: &str = "browser-bold-italic";

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
    pub fn new(tokens: &[Token], width: f32, rtl: bool, ctx: &egui::Context) -> Self {
        let mut layout = Self {
            display_list: Vec::new(),
            width,
            rtl,
            cursor_x: HSTEP,
            cursor_y: VSTEP,
            weight: "normal",
            style: "roman",
            size: 12.0,
            line: Vec::new(),
        };

        for tok in tokens {
            layout.token(tok, ctx);
        }

        layout.flush(ctx);
        layout
    }

    fn token(&mut self, tok: &Token, ctx: &egui::Context) {
        match tok {
            Token::Text(_) => {
                for word in extract_text(std::slice::from_ref(tok)).split_whitespace() {
                    self.word(word, ctx);
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
                    self.newline(ctx);
                    self.cursor_y += VSTEP;
                }
                _ => {
                    let normalized = tag.trim().to_ascii_lowercase();
                    if matches!(normalized.as_str(), "br" | "br/" | "/div") {
                        self.newline(ctx);
                    }
                }
            },
        }
    }

    fn word(&mut self, word: &str, ctx: &egui::Context) {
        let bold = self.weight == "bold";
        let italic = self.style == "italic";
        let w = measure_text(ctx, word, bold, italic, self.size);

        if self.cursor_x + w > self.width - HSTEP - SCROLLBAR_WIDTH {
            self.flush(ctx);
        }

        self.line
            .push((self.cursor_x, word.to_string(), bold, italic, self.size));
        self.cursor_x += measure_text(ctx, &format!("{word} "), bold, italic, self.size);
    }

    fn newline(&mut self, ctx: &egui::Context) {
        self.flush(ctx);
    }

    fn flush(&mut self, ctx: &egui::Context) {
        if self.line.is_empty() {
            return;
        }

        let metrics: Vec<FontMetrics> = self
            .line
            .iter()
            .map(|(_, _, bold, italic, size)| measure_metrics(ctx, *bold, *italic, *size))
            .collect();
        let max_ascent = metrics
            .iter()
            .map(|metric| metric.ascent)
            .fold(0.0, f32::max);
        let baseline = self.cursor_y + 1.25 * max_ascent;
        let shift = if self.rtl {
            (self.width - HSTEP - SCROLLBAR_WIDTH - self.measure_line(ctx)).max(HSTEP) - HSTEP
        } else {
            0.0
        };

        for (x, word, bold, italic, size) in &self.line {
            let y = baseline - measure_metrics(ctx, *bold, *italic, *size).ascent;
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

    fn measure_line(&self, ctx: &egui::Context) -> f32 {
        let (last_x, last_word, last_bold, last_italic, last_size) =
            self.line.last().unwrap();
        last_x + measure_text(ctx, last_word, *last_bold, *last_italic, *last_size) - HSTEP
    }
}

struct FontMetrics {
    ascent: f32,
    descent: f32,
}

pub fn layout_word(
    ctx: &egui::Context,
    text: &str,
    bold: bool,
    italic: bool,
    size: f32,
    color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    let mut format = TextFormat {
        font_id: egui::FontId {
            size,
            family: font_family_for(bold, italic),
        },
        color,
        ..Default::default()
    };
    format.italics = italic;
    job.append(text, 0.0, format);
    let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
    if bold {
        galley
    } else {
        galley
    }
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

fn measure_text(ctx: &egui::Context, text: &str, bold: bool, italic: bool, size: f32) -> f32 {
    layout_word(ctx, text, bold, italic, size, egui::Color32::WHITE)
        .size()
        .x
}

fn measure_metrics(ctx: &egui::Context, bold: bool, italic: bool, size: f32) -> FontMetrics {
    let height = layout_word(ctx, "Ag", bold, italic, size, egui::Color32::WHITE)
        .size()
        .y;
    FontMetrics {
        ascent: height * 0.8,
        descent: height * 0.2,
    }
}
