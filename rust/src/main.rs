mod graphics;
mod network;

fn main() -> eframe::Result<()> {
    let mut rtl = false;
    let mut url = None;

    for arg in std::env::args().skip(1) {
        if arg == "--rtl" {
            rtl = true;
        } else if url.is_none() {
            url = Some(arg);
        }
    }

    graphics::run(url, rtl)
}
