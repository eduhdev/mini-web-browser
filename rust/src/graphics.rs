use eframe::egui;
use resvg::{tiny_skia::Transform, usvg};
use signal_hook::consts::signal::{SIGINT, SIGTSTP};
use signal_hook::flag;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use tiny_skia::Pixmap;
use unicode_segmentation::UnicodeSegmentation;

use crate::network::{lex, Url};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const HSTEP: f32 = 13.0;
const VSTEP: f32 = 18.0;
const SCROLL_STEP: f32 = 100.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const EMOJI_SIZE: u32 = 18;
static INTERRUPTED: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));
static OPENMOJI_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("openmoji")
});

pub fn run(url: Option<String>) -> eframe::Result<()> {
    let _ = flag::register(SIGINT, INTERRUPTED.clone());
    let _ = flag::register(SIGTSTP, INTERRUPTED.clone());

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
    display_list: Vec<(f32, f32, String)>,
    scroll: f32,
    width: f32,
    height: f32,
    emoji_cache: HashMap<String, egui::TextureHandle>,
}

impl Browser {
    fn new(url: Option<String>) -> Self {
        let mut browser = Self {
            text: String::new(),
            display_list: Vec::new(),
            scroll: 0.0,
            width: WIDTH,
            height: HEIGHT,
            emoji_cache: HashMap::new(),
        };

        if let Some(url) = url {
            browser.load(Url::new(&url));
        }

        browser
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let font_id = egui::FontId::proportional(16.0);
        let color = ui.visuals().text_color();
        let ctx = ui.ctx().clone();

        for (x, y, token) in self.display_list.clone() {
            if y > self.scroll + self.height {
                continue;
            }
            if y + VSTEP < self.scroll {
                continue;
            }

            if let Some(texture) = self.load_emoji(&ctx, &token) {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(x, y - self.scroll),
                    egui::vec2(EMOJI_SIZE as f32, EMOJI_SIZE as f32),
                );
                painter.image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                painter.text(
                    egui::pos2(x, y - self.scroll),
                    egui::Align2::LEFT_TOP,
                    token,
                    font_id.clone(),
                    color,
                );
            }
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

    fn load_emoji(&mut self, ctx: &egui::Context, token: &str) -> Option<egui::TextureHandle> {
        if self.emoji_cache.contains_key(token) {
            return self.emoji_cache.get(token).cloned();
        }

        let texture = load_emoji_texture(ctx, token)?;
        self.emoji_cache.insert(token.to_owned(), texture);
        self.emoji_cache.get(token).cloned()
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
        if INTERRUPTED.load(Ordering::Relaxed) {
            process::exit(0);
        }

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

fn layout(text: &str, width: f32) -> Vec<(f32, f32, String)> {
    let mut display_list = Vec::new();
    let mut cursor_x = HSTEP;
    let mut cursor_y = VSTEP;

    for token in tokenize(text) {
        if token == "\n" {
            cursor_x = HSTEP;
            cursor_y += 1.5 * VSTEP;
            continue;
        }

        display_list.push((cursor_x, cursor_y, token));
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

    for (name, path) in system_font_candidates() {
        if !Path::new(path).exists() {
            continue;
        }

        let Ok(bytes) = fs::read(path) else {
            continue;
        };

        fonts.font_data.insert(
            (*name).to_owned(),
            egui::FontData::from_owned(bytes).into(),
        );
    }

    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional.insert(0, "system-ui".to_owned());
    proportional.push("apple-color-emoji".to_owned());

    let monospace = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    monospace.push("system-ui".to_owned());
    monospace.push("apple-color-emoji".to_owned());

    ctx.set_fonts(fonts);
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();

    for grapheme in text.graphemes(true) {
        if grapheme == "\n" {
            tokens.push("\n".to_owned());
            continue;
        }

        tokens.push(grapheme.to_owned());
    }

    tokens
}

fn load_emoji_texture(ctx: &egui::Context, token: &str) -> Option<egui::TextureHandle> {
    let path = emoji_path_for(token)?;
    let svg = fs::read(&path).ok()?;
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg, &options).ok()?;
    let mut pixmap = Pixmap::new(EMOJI_SIZE, EMOJI_SIZE)?;
    let size = tree.size();
    let scale = (EMOJI_SIZE as f32 / size.width()).min(EMOJI_SIZE as f32 / size.height());
    let scaled_width = size.width() * scale;
    let scaled_height = size.height() * scale;
    let dx = (EMOJI_SIZE as f32 - scaled_width) / 2.0;
    let dy = (EMOJI_SIZE as f32 - scaled_height) / 2.0;
    let transform = Transform::from_scale(scale, scale).post_translate(dx, dy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = egui::ColorImage::from_rgba_unmultiplied(
        [EMOJI_SIZE as usize, EMOJI_SIZE as usize],
        pixmap.data(),
    );

    Some(ctx.load_texture(
        format!("emoji-{token}"),
        image,
        egui::TextureOptions::LINEAR,
    ))
}

fn emoji_path_for(token: &str) -> Option<PathBuf> {
    if token == "\n" || token.is_empty() {
        return None;
    }

    let codepoints = token
        .chars()
        .map(|c| format!("{:X}", c as u32))
        .collect::<Vec<_>>()
        .join("-");
    let path = OPENMOJI_DIR.join(format!("{codepoints}.svg"));
    path.exists().then_some(path)
}

fn system_font_candidates() -> &'static [(&'static str, &'static str)] {
    &[
        ("system-ui", "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        ("apple-color-emoji", "/System/Library/Fonts/Apple Color Emoji.ttc"),
        ("system-ui", "/System/Library/Fonts/Supplemental/AppleGothic.ttf"),
        ("system-ui", "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"),
        ("system-ui", "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
        ("system-ui", "C:\\Windows\\Fonts\\arialuni.ttf"),
        ("system-ui", "C:\\Windows\\Fonts\\msgothic.ttc"),
    ]
}
