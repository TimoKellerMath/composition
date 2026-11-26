//! The notes of the selected result, rendered by LilyPond to PNG pages in the background.

use std::path::PathBuf;
use std::thread::JoinHandle;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use fux_export::{Graphic, render};

/// What to render: the LilyPond source, and a key that changes with it.
pub struct Request {
    pub key: u64,
    pub source: String,
}

/// A rendering in progress: its key and the thread that produces the pages.
type Rendering = (u64, JoinHandle<Result<Vec<ColorImage>, String>>);

/// The rendered pages of the current result.
pub struct Preview {
    pages: Vec<TextureHandle>,
    /// Key of the pages shown, and of the rendering in progress.
    shown: Option<u64>,
    running: Option<Rendering>,
    /// The latest request while another one runs.
    waiting: Option<Request>,
    pub error: Option<String>,
    dir: PathBuf,
    counter: u64,
}

impl Default for Preview {
    fn default() -> Self {
        Preview {
            pages: Vec::new(),
            shown: None,
            running: None,
            waiting: None,
            error: None,
            dir: std::env::temp_dir().join(format!("fux-gui-{}", std::process::id())),
            counter: 0,
        }
    }
}

impl Preview {
    /// Shows the rendering of `request`, starting it unless it is shown or running already.
    pub fn request(&mut self, request: Request) {
        let current = self.running.as_ref().map(|(key, _)| *key).or(self.shown);
        if current == Some(request.key) {
            return;
        }
        if self.running.is_some() {
            self.waiting = Some(request);
            return;
        }
        self.start(request);
    }

    fn start(&mut self, request: Request) {
        self.counter += 1;
        let base = self.dir.join(format!("preview-{}", self.counter));
        let dir = self.dir.clone();
        let handle = std::thread::spawn(move || render_pages(&dir, &base, &request.source));
        self.running = Some((request.key, handle));
    }

    /// Clears the pages, e.g. when there is no result to show.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.shown = None;
        self.waiting = None;
        self.error = None;
    }

    pub fn is_rendering(&self) -> bool {
        self.running.is_some()
    }

    /// Whether a request with `key` would change nothing ([`Preview::request`]): its pages are
    /// shown or being rendered, or they wait to be. Its source need not be made then.
    pub fn knows(&self, key: u64) -> bool {
        let running = self.running.as_ref().map(|(running, _)| *running);
        let waiting = self.waiting.as_ref().map(|request| request.key);
        running.or(self.shown) == Some(key) || (running.is_some() && waiting == Some(key))
    }

    /// Takes a finished rendering over as textures; starts the waiting request.
    pub fn poll(&mut self, ctx: &egui::Context) {
        if !self
            .running
            .as_ref()
            .is_some_and(|(_, handle)| handle.is_finished())
        {
            return;
        }
        let (key, handle) = self.running.take().expect("checked");
        let result = handle
            .join()
            .unwrap_or_else(|_| Err("LilyPond: internal error".into()));
        // A newer request makes this result obsolete.
        if let Some(request) = self.waiting.take() {
            self.start(request);
            return;
        }
        match result {
            Ok(images) => {
                self.pages = images
                    .into_iter()
                    .enumerate()
                    .map(|(i, image)| {
                        ctx.load_texture(format!("page-{key}-{i}"), image, TextureOptions::LINEAR)
                    })
                    .collect();
                self.error = None;
            }
            Err(error) => {
                self.pages.clear();
                self.error = Some(error);
            }
        }
        self.shown = Some(key);
    }

    /// The pages, scaled to `width` times `zoom`.
    pub fn show(&self, ui: &mut egui::Ui, zoom: f32) {
        let width = ui.available_width() * zoom;
        for page in &self.pages {
            let size = page.size_vec2();
            let scaled = egui::vec2(width, width * size.y / size.x);
            ui.add(egui::Image::new((page.id(), scaled)));
            ui.add_space(8.0);
        }
    }

    pub fn has_pages(&self) -> bool {
        !self.pages.is_empty()
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Writes the source, lets LilyPond render it, reads the pages and removes the files.
fn render_pages(
    dir: &std::path::Path,
    base: &std::path::Path,
    source: &str,
) -> Result<Vec<ColorImage>, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let ly = base.with_extension("ly");
    std::fs::write(&ly, source).map_err(|e| e.to_string())?;
    let rendered = render(&ly, base, Graphic::Png);
    let _ = std::fs::remove_file(&ly);
    let files = rendered.map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    for file in &files {
        let decoded = image::open(file).map_err(|e| format!("{}: {e}", file.display()));
        let _ = std::fs::remove_file(file);
        let rgba = decoded?.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        pages.push(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()));
    }
    Ok(pages)
}
