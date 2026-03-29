use eframe::egui;
use std::fs;
use std::path::Path;

use crate::network::{lex, Url};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const HSTEP: f32 = 13.0;
const VSTEP: f32 = 18.0;
const SCROLL_STEP: f32 = 100.0;
const SCROLLBAR_WIDTH: f32 = 8.0;

pub fn run(url: Option<String>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([WIDTH, HEIGHT]),
        ..Default::default()
    };
    eframe::run_native(
        "Browser",
        options,
        Box::new(|cc| {
            install_system_font(&cc.egui_ctx);
            Ok(Box::new(Browser::new(url)))
        }),
    )
}

struct Browser {
    text: String,
    display_list: Vec<(f32, f32, char)>,
    scroll: f32,
    width: f32,
    height: f32,
}

impl Browser {
    fn new(url: Option<String>) -> Self {
        let mut browser = Self {
            text: String::new(),
            display_list: Vec::new(),
            scroll: 0.0,
            width: WIDTH,
            height: HEIGHT,
        };

        if let Some(url) = url {
            browser.load(Url::new(&url));
        }

        browser
    }

    fn draw(&self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let font_id = egui::FontId::proportional(16.0);
        let color = ui.visuals().text_color();

        for &(x, y, c) in &self.display_list {
            if y > self.scroll + self.height {
                continue;
            }
            if y + VSTEP < self.scroll {
                continue;
            }

            painter.text(
                egui::pos2(x, y - self.scroll),
                egui::Align2::LEFT_TOP,
                c,
                font_id.clone(),
                color,
            );
        }

        self.draw_scrollbar(painter);
    }

    fn load(&mut self, url: Url) {
        let body = url.request();
        self.text = lex(&body);
        self.display_list = layout(&self.text, self.width);
        self.scroll = 0.0;
    }

    fn scrollby(&mut self, amount: f32) {
        let new_scroll = (self.scroll + amount).clamp(0.0, self.max_scroll());
        if new_scroll == self.scroll {
            return;
        }
        self.scroll = new_scroll;
    }

    fn document_height(&self) -> f32 {
        self.display_list
            .last()
            .map(|(_, y, _)| *y + VSTEP)
            .unwrap_or(self.height)
    }

    fn max_scroll(&self) -> f32 {
        (self.document_height() - self.height).max(0.0)
    }

    fn draw_scrollbar(&self, painter: &egui::Painter) {
        let document_height = self.document_height();
        if document_height <= self.height {
            return;
        }

        let top = self.scroll / document_height * self.height;
        let bottom = (self.scroll + self.height) / document_height * self.height;
        let rect = egui::Rect::from_min_max(
            egui::pos2(self.width - SCROLLBAR_WIDTH, top),
            egui::pos2(self.width, bottom),
        );
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(173, 216, 230));
    }
}

impl eframe::App for Browser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let new_size = ctx.input(|input| input.content_rect().size());
        if new_size.x != self.width || new_size.y != self.height {
            self.width = new_size.x;
            self.height = new_size.y;
            if !self.text.is_empty() {
                self.display_list = layout(&self.text, self.width);
                self.scroll = self.scroll.min(self.max_scroll());
            }
        }

        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowDown) {
                self.scrollby(SCROLL_STEP);
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                self.scrollby(-SCROLL_STEP);
            }

            let delta_y = input.raw_scroll_delta.y;
            if delta_y != 0.0 {
                self.scrollby(-delta_y);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw(ui);
        });
    }
}

fn layout(text: &str, width: f32) -> Vec<(f32, f32, char)> {
    let mut display_list = Vec::new();
    let mut cursor_x = HSTEP;
    let mut cursor_y = VSTEP;

    for c in text.chars() {
        if c == '\n' {
            cursor_x = HSTEP;
            cursor_y += 1.5 * VSTEP;
            continue;
        }

        display_list.push((cursor_x, cursor_y, c));
        cursor_x += HSTEP;

        if cursor_x >= width - HSTEP {
            cursor_x = HSTEP;
            cursor_y += VSTEP;
        }
    }

    display_list
}

fn install_system_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    for path in system_font_candidates() {
        if !Path::new(path).exists() {
            continue;
        }

        let Ok(bytes) = fs::read(path) else {
            continue;
        };

        fonts.font_data.insert(
            "system-ui".to_owned(),
            egui::FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "system-ui".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("system-ui".to_owned());

        ctx.set_fonts(fonts);
        return;
    }
}

fn system_font_candidates() -> &'static [&'static str] {
    &[
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "C:\\Windows\\Fonts\\arialuni.ttf",
        "C:\\Windows\\Fonts\\msgothic.ttc",
    ]
}
