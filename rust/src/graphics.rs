use eframe::egui;
use signal_hook::consts::signal::{SIGINT, SIGTSTP};
use signal_hook::flag;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use crate::constants::{HEIGHT, SCROLL_STEP, SCROLLBAR_WIDTH, VSTEP, WIDTH};
use crate::emoji::EmojiCache;
use crate::layout::{DocumentLayout, DrawCommand, FontCache};
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
    display_list: Vec<DrawCommand>,
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

        for command in &self.display_list {
            if command.top() > self.scroll + self.height {
                continue;
            }
            if command.bottom() < self.scroll {
                continue;
            }
            command.execute(
                self.scroll,
                painter,
                &ctx,
                color,
                &mut self.emoji_cache,
                &mut self.font_cache,
            );
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
        let mut document = DocumentLayout::new(nodes, self.width, self.rtl);
        document.layout(ctx, &mut self.font_cache);
        self.display_list.clear();
        paint_tree(Paintable::Document(&document), &mut self.display_list);
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
            .map(|command| command.bottom() + VSTEP)
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

fn paint_tree(layout_object: Paintable<'_>, display_list: &mut Vec<DrawCommand>) {
    display_list.extend(layout_object.paint());
    for child in layout_object.children() {
        paint_tree(child, display_list);
    }
}

enum Paintable<'a> {
    Document(&'a DocumentLayout),
    Block(&'a crate::layout::BlockLayout),
}

impl<'a> Paintable<'a> {
    fn paint(&self) -> Vec<DrawCommand> {
        match self {
            Self::Document(layout) => layout.paint(),
            Self::Block(layout) => layout.paint(),
        }
    }

    fn children(&self) -> Vec<Paintable<'a>> {
        match self {
            Self::Document(layout) => layout.children.iter().map(Paintable::Block).collect(),
            Self::Block(layout) => layout.children.iter().map(Paintable::Block).collect(),
        }
    }
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
