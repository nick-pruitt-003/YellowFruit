//! `YF_GUI_SHOT=<dir>`: show each page in turn, save it as `<dir>/<page>.png`, then quit.
//! Used to check the UI without a person at the screen.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::Page;

pub struct Shooter {
    dir: PathBuf,
    next: usize,
    frames: u32,
    waiting: bool,
    got: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,
}

impl Shooter {
    pub fn from_env() -> Option<Self> {
        let dir = PathBuf::from(std::env::var_os("YF_GUI_SHOT")?);
        std::fs::create_dir_all(&dir).ok()?;
        Some(Shooter { dir, next: 0, frames: 0, waiting: false, got: Default::default() })
    }

    /// Advance one frame. Returns false when every page has been saved.
    pub fn step(&mut self, ctx: &egui::Context, page: &mut Page) -> bool {
        let image = self.got.lock().unwrap().take();
        if let Some(img) = image {
            let path = self.dir.join(format!("{}.png", Page::ALL[self.next].label().to_lowercase()));
            if let Err(e) = save_png(&path, &img) {
                eprintln!("couldn't save {}: {e}", path.display());
            }
            self.next += 1;
            self.waiting = false;
            self.frames = 0;
        }
        if self.next >= Page::ALL.len() {
            return false;
        }
        *page = Page::ALL[self.next];
        ctx.request_repaint();
        self.frames += 1;
        // let the page lay out for a few frames before capturing
        if !self.waiting && self.frames > 5 {
            let got = self.got.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                *got.lock().unwrap() = Some(img);
            })));
            self.waiting = true;
        }
        true
    }
}

fn save_png(path: &std::path::Path, img: &egui::ColorImage) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(img.as_raw()).map_err(|e| e.to_string())
}
