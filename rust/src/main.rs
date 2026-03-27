mod graphics;
mod network;

fn main() -> eframe::Result<()> {
    let url = std::env::args().nth(1);
    graphics::run(url)
}
