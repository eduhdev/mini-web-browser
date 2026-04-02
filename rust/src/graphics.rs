use eframe::egui;
use signal_hook::consts::signal::{SIGINT, SIGTSTP};
use signal_hook::flag;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use crate::constants::{EMOJI_SIZE, HEIGHT, SCROLL_STEP, SCROLLBAR_WIDTH, VSTEP, WIDTH};
use crate::emoji::EmojiCache;
use crate::layout::{DisplayItem, FontCache, Layout};
use crate::network::{default_file_url, Url};
use crate::parser::{print_tree, HtmlParser, Node};

static INTERRUPTED: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

pub fn run(url: Option<String>, rtl: bool) -> eframe::Result<()> {
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
            Ok(Box::new(Browser::new(url, rtl)))
        }),
    )
}

struct Browser {
    nodes: Option<Node>,
    display_list: Vec<DisplayItem>,
    scroll: f32,
    width: f32,
    height: f32,
    rtl: bool,
    emoji_cache: EmojiCache,
    font_cache: FontCache,
}

impl Browser {
    fn new(url: Option<String>, rtl: bool) -> Self {
        let mut browser = Self {
            nodes: None,
            display_list: Vec::new(),
            scroll: 0.0,
            width: WIDTH,
            height: HEIGHT,
            rtl,
            emoji_cache: EmojiCache::new(),
            font_cache: FontCache::new(),
        };

        let url = url.unwrap_or_else(default_file_url);
        browser.load(Url::new(&url));

        browser
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let color = ui.visuals().text_color();
        let ctx = ui.ctx().clone();

        for (x, y, token, bold, italic, size) in self.display_list.clone() {
            if y > self.scroll + self.height {
                continue;
            }
            if y + VSTEP < self.scroll {
                continue;
            }

            if let Some(texture) = self.emoji_cache.load(&ctx, &token) {
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
                let galley = self.font_cache.layout_word(&ctx, &token, bold, italic, size);
                let pos = egui::pos2(x, y - self.scroll);
                painter.galley(pos, galley, color);
            }
        }

        self.draw_scrollbar(painter);
    }

    fn load(&mut self, url: Url) {
        let body = url.request();
        self.nodes = Some(HtmlParser::new(&body).parse());
        if let Some(nodes) = &self.nodes {
            print_tree(nodes);
        }
        self.display_list.clear();
        self.scroll = 0.0;
    }

    fn relayout(&mut self, ctx: &egui::Context) {
        let Some(nodes) = &self.nodes else {
            return;
        };
        self.display_list =
            Layout::new(nodes, self.width, self.rtl, ctx, &mut self.font_cache).display_list;
        self.scroll = self.scroll.min(self.max_scroll());
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
            .map(|(_, y, _, _, _, _)| *y + VSTEP)
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
            if self.nodes.is_some() {
                self.relayout(ctx);
            }
        }

        if self.display_list.is_empty() && self.nodes.is_some() {
            self.relayout(ctx);
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

fn install_system_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    load_font_data(
        &mut fonts,
        "browser-regular",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    );
    load_font_data(
        &mut fonts,
        "browser-bold",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    );
    load_font_data(
        &mut fonts,
        "browser-italic",
        "/System/Library/Fonts/Supplemental/Arial Italic.ttf",
    );
    load_font_data(
        &mut fonts,
        "browser-bold-italic",
        "/System/Library/Fonts/Supplemental/Arial Bold Italic.ttf",
    );
    load_font_data(
        &mut fonts,
        "browser-unicode",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    );
    load_font_data(
        &mut fonts,
        "apple-color-emoji",
        "/System/Library/Fonts/Apple Color Emoji.ttc",
    );

    set_family(
        &mut fonts,
        "browser-regular",
        &["browser-regular", "browser-unicode", "apple-color-emoji"],
    );
    set_family(
        &mut fonts,
        "browser-bold",
        &["browser-bold", "browser-unicode", "apple-color-emoji"],
    );
    set_family(
        &mut fonts,
        "browser-italic",
        &["browser-italic", "browser-unicode", "apple-color-emoji"],
    );
    set_family(
        &mut fonts,
        "browser-bold-italic",
        &["browser-bold-italic", "browser-unicode", "apple-color-emoji"],
    );

    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional.insert(0, "browser-regular".to_owned());
    proportional.push("browser-unicode".to_owned());
    proportional.push("apple-color-emoji".to_owned());

    let monospace = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    monospace.push("browser-regular".to_owned());
    monospace.push("browser-unicode".to_owned());
    monospace.push("apple-color-emoji".to_owned());

    ctx.set_fonts(fonts);
}

fn load_font_data(fonts: &mut egui::FontDefinitions, name: &str, path: &str) {
    let path = std::path::Path::new(path);
    if !path.exists() {
        return;
    }

    let Ok(bytes) = std::fs::read(path) else {
        return;
    };

    fonts.font_data.insert(
        name.to_owned(),
        egui::FontData::from_owned(bytes).into(),
    );
}

fn set_family(fonts: &mut egui::FontDefinitions, family_name: &str, fonts_in_family: &[&str]) {
    fonts.families.insert(
        egui::FontFamily::Name(std::sync::Arc::<str>::from(family_name)),
        fonts_in_family.iter().map(|name| (*name).to_owned()).collect(),
    );
}
