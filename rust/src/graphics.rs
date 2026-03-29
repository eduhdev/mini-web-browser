use eframe::egui;
use std::fs;
use std::path::Path;

use crate::network::{lex, Url};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const HSTEP: f32 = 13.0;
const VSTEP: f32 = 18.0;
const SCROLL_STEP: f32 = 100.0;

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
    display_list: Vec<(f32, f32, char)>,
    scroll: f32,
}

impl Browser {
    fn new(url: Option<String>) -> Self {
        let mut browser = Self {
            display_list: Vec::new(),
            scroll: 0.0,
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
            if y > self.scroll + HEIGHT {
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
    }

    fn load(&mut self, url: Url) {
        let body = url.request();
        let text = lex(&body);
        self.display_list = layout(&text);
        self.scroll = 0.0;
    }
}

impl eframe::App for Browser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowDown) {
                self.scroll += SCROLL_STEP;
            }
            if input.key_pressed(egui::Key::ArrowUp) && self.scroll > 0.0 {
                self.scroll = (self.scroll - SCROLL_STEP).max(0.0);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw(ui);
        });
    }
}

fn layout(text: &str) -> Vec<(f32, f32, char)> {
    let mut display_list = Vec::new();
    let mut cursor_x = HSTEP;
    let mut cursor_y = VSTEP;

    for c in text.chars() {
        display_list.push((cursor_x, cursor_y, c));
        cursor_x += HSTEP;

        if cursor_x >= WIDTH - HSTEP {
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
