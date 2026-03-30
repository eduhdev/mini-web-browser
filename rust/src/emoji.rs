use eframe::egui;
use resvg::{tiny_skia::Transform, usvg};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tiny_skia::Pixmap;

use crate::constants::EMOJI_SIZE;

static OPENMOJI_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("openmoji")
});

pub struct EmojiCache {
    cache: HashMap<String, egui::TextureHandle>,
}

impl EmojiCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn load(&mut self, ctx: &egui::Context, token: &str) -> Option<egui::TextureHandle> {
        if self.cache.contains_key(token) {
            return self.cache.get(token).cloned();
        }

        let texture = load_emoji_texture(ctx, token)?;
        self.cache.insert(token.to_owned(), texture);
        self.cache.get(token).cloned()
    }
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
