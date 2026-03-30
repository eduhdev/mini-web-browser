use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};

use crate::constants::{FONT_SIZE, HSTEP, SCROLLBAR_WIDTH, VSTEP};
use crate::network::{extract_text, Token};

pub type DisplayItem = (f32, f32, String, bool, bool);

pub struct Layout {
    pub display_list: Vec<DisplayItem>,
    width: f32,
    rtl: bool,
    cursor_x: f32,
    cursor_y: f32,
    weight: &'static str,
    style: &'static str,
    line: Vec<(String, bool, bool)>,
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
        let w = measure_text(ctx, word, bold, italic);

        if self.cursor_x + w > self.width - HSTEP - SCROLLBAR_WIDTH {
            self.flush(ctx);
            self.cursor_y += self.line_height();
            self.cursor_x = HSTEP;
        }

        self.line.push((word.to_string(), bold, italic));
        self.cursor_x += measure_text(ctx, &format!("{word} "), bold, italic);
    }

    fn newline(&mut self, ctx: &egui::Context) {
        self.flush(ctx);
        self.cursor_y += self.line_height();
        self.cursor_x = HSTEP;
    }

    fn flush(&mut self, ctx: &egui::Context) {
        if self.line.is_empty() {
            return;
        }

        let mut cursor_x = if self.rtl {
            (self.width - HSTEP - SCROLLBAR_WIDTH - self.measure_line(ctx)).max(HSTEP)
        } else {
            HSTEP
        };

        for (word, bold, italic) in &self.line {
            self.display_list
                .push((cursor_x, self.cursor_y, word.clone(), *bold, *italic));
            cursor_x += measure_text(ctx, &format!("{word} "), *bold, *italic);
        }

        self.line.clear();
    }

    fn measure_line(&self, ctx: &egui::Context) -> f32 {
        let mut width = 0.0;
        for (i, (word, bold, italic)) in self.line.iter().enumerate() {
            width += measure_text(ctx, word, *bold, *italic);
            if i < self.line.len() - 1 {
                width += measure_text(ctx, " ", *bold, *italic);
            }
        }
        width
    }

    fn line_height(&self) -> f32 {
        FONT_SIZE * 1.25
    }
}

pub fn layout_word(
    ctx: &egui::Context,
    text: &str,
    bold: bool,
    italic: bool,
    color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    let mut format = TextFormat {
        font_id: egui::FontId::proportional(FONT_SIZE),
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

fn measure_text(ctx: &egui::Context, text: &str, bold: bool, italic: bool) -> f32 {
    layout_word(ctx, text, bold, italic, egui::Color32::WHITE)
        .size()
        .x
}
