//! YellowFruit egui front end. Read-only for now: open a tournament file and browse the General,
//! Teams, Standings and Games pages; the stat report opens in the system browser.
//! All data and numbers come from yf-core, which is parity-tested against the TS app.

mod pages;
mod shot;

use std::path::{Path, PathBuf};

use eframe::egui;
use yf_core::Tournament;

/// The TS app version this port matches (written into files on save)
pub const CUR_YF_VERSION: &str = "4.0.18";

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--smoke") {
        std::process::exit(smoke(&args[2..]));
    }
    let initial = std::env::args().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("YellowFruit")
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([700.0, 450.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "YellowFruit",
        options,
        Box::new(move |cc| {
            pages::apply_theme(&cc.egui_ctx);
            let mut app = App { shot: shot::Shooter::from_env(), ..Default::default() };
            if let Some(p) = initial {
                app.open(&p);
            }
            Ok(Box::new(app))
        }),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    General,
    Teams,
    Standings,
    Games,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::General, Page::Teams, Page::Standings, Page::Games];
    pub fn label(self) -> &'static str {
        match self {
            Page::General => "General",
            Page::Teams => "Teams",
            Page::Standings => "Standings",
            Page::Games => "Games",
        }
    }
}

pub struct OpenFile {
    pub path: PathBuf,
    pub format: yf_core::Format,
    pub tournament: Tournament,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct App {
    file: Option<OpenFile>,
    page: Page,
    /// Last error (failed open, failed report)
    error: Option<String>,
    status: Option<String>,
    shot: Option<shot::Shooter>,
}

impl App {
    fn open(&mut self, path: &Path) {
        self.error = None;
        self.status = None;
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                self.error = Some(format!("Couldn't read {}: {e}", path.display()));
                return;
            }
        };
        match yf_core::open_file(&contents, CUR_YF_VERSION) {
            (format, Ok(loaded)) => {
                let mut tournament = loaded.tournament;
                // as TournamentManager.compileStats does
                tournament.compile_stats(false, true);
                self.file = Some(OpenFile { path: path.to_path_buf(), format, tournament, warnings: loaded.warnings });
            }
            (_, Err(e)) => self.error = Some(format!("Couldn't open {}: {e}", file_name(path))),
        }
    }

    fn pick_and_open(&mut self) {
        let picked = rfd::FileDialog::new()
            .add_filter("YellowFruit tournament", &["yft"])
            .add_filter("All files", &["*"])
            .pick_file();
        if let Some(p) = picked {
            self.open(&p);
        }
    }

    /// Write the six report pages to a temp folder and open the standings page in the browser
    fn open_report(&mut self) {
        let Some(f) = self.file.as_mut() else { return };
        let pages = match yf_core::html::generate_all(&mut f.tournament, None) {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(format!("Couldn't make the stat report: {e}"));
                return;
            }
        };
        // generate_all recompiles stats with the full report on; restore the in-app ones
        f.// as TournamentManager.compileStats does
                tournament.compile_stats(false, true);
        let dir = std::env::temp_dir().join("yf-gui-report");
        let files = [
            ("standings.html", &pages.standings),
            ("individuals.html", &pages.individuals),
            ("games.html", &pages.scoreboard),
            ("teamdetail.html", &pages.team_details),
            ("playerdetail.html", &pages.player_details),
            ("rounds.html", &pages.round_report),
        ];
        let written = std::fs::create_dir_all(&dir)
            .and_then(|_| files.iter().try_for_each(|(name, html)| std::fs::write(dir.join(name), html)));
        if let Err(e) = written {
            self.error = Some(format!("Couldn't write the stat report: {e}"));
            return;
        }
        match open::that(dir.join("standings.html")) {
            Ok(()) => self.status = Some(format!("Stat report opened from {}", dir.display())),
            Err(e) => self.error = Some(format!("Couldn't open the browser: {e}")),
        }
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(mut s) = self.shot.take() {
            let keep = s.step(&ctx, &mut self.page);
            if keep {
                self.shot = Some(s);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        // Drag a file onto the window to open it
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf())) {
            self.open(&p);
        }
        // Cmd/Ctrl+O opens a file
        if ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O))) {
            self.pick_and_open();
        }

        let title = match &self.file {
            Some(f) => format!("{} - YellowFruit", file_name(&f.path)),
            None => "YellowFruit".to_string(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::Panel::top("nav").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Open…").on_hover_text("Open a .yft file (Cmd/Ctrl+O)").clicked() {
                    self.pick_and_open();
                }
                ui.separator();
                let enabled = self.file.is_some();
                for p in Page::ALL {
                    ui.add_enabled_ui(enabled, |ui| {
                        if ui.selectable_label(self.page == p, p.label()).clicked() {
                            self.page = p;
                        }
                    });
                }
                ui.separator();
                if ui.add_enabled(enabled, egui::Button::new("Stat report ↗")).on_hover_text("Open the full report in your browser").clicked() {
                    self.open_report();
                }
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(e) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                } else if let Some(s) = &self.status {
                    ui.label(s);
                } else if let Some(f) = &self.file {
                    ui.label(format!("{} ({} format)", f.path.display(), f.format.as_str()));
                    if !f.warnings.is_empty() {
                        ui.colored_label(ui.visuals().warn_fg_color, format!("· {} warning(s) on open", f.warnings.len()))
                            .on_hover_text(f.warnings.join("\n"));
                    }
                } else {
                    ui.label("No file open");
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| match &self.file {
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.heading("YellowFruit");
                    ui.label("Open a .yft file, or drop one onto this window.");
                    ui.add_space(8.0);
                    if ui.button("Open…").clicked() {
                        self.pick_and_open();
                    }
                });
            }
            Some(f) => {
                egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| match self.page {
                    Page::General => pages::general(ui, f),
                    Page::Teams => pages::teams(ui, &f.tournament),
                    Page::Standings => pages::standings(ui, &f.tournament),
                    Page::Games => pages::games(ui, &f.tournament),
                });
            }
        });
    }
}

/// `yf-gui --smoke <files>`: draw every page of every file offscreen and report files that panic.
/// Files that fail to load are skipped (the parity check covers loading).
fn smoke(files: &[String]) -> i32 {
    let (mut ok, mut skipped, mut failed) = (0, 0, 0);
    for path in files {
        let mut app = App::default();
        app.open(Path::new(path));
        let Some(f) = app.file.as_ref() else {
            skipped += 1;
            continue;
        };
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let ctx = egui::Context::default();
            pages::apply_theme(&ctx);
            for page in Page::ALL {
                let mut out = ctx.run_ui(Default::default(), |ui| match page {
                    Page::General => pages::general(ui, f),
                    Page::Teams => pages::teams(ui, &f.tournament),
                    Page::Standings => pages::standings(ui, &f.tournament),
                    Page::Games => pages::games(ui, &f.tournament),
                });
                out.textures_delta.clear();
            }
        }));
        match res {
            Ok(()) => ok += 1,
            Err(_) => {
                failed += 1;
                eprintln!("PANIC: {path}");
            }
        }
    }
    println!("{} files: {ok} drew all pages, {skipped} didn't load, {failed} panicked", files.len());
    i32::from(failed > 0)
}
