mod menu;
mod mpv;
mod overlay;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use chrono::{Local, Timelike};

use menu::{Category, Menu};
use mpv::Mpv;

const MENU_OVERLAY: u32 = 1;
const SCREEN_OVERLAY: u32 = 2;
const VIDEO_EXTENSIONS: [&str; 7] = ["mp4", "mkv", "webm", "mov", "m4v", "avi", "gif"];

/// Actions de l'utilisateur, indépendantes de leur source (clavier aujourd'hui, bouton demain).
#[derive(Clone, Copy, Debug)]
pub enum Input {
    Previous,
    Next,
    Select,
    Back,
}

impl Input {
    pub fn parse(name: &str) -> Option<Input> {
        match name {
            "previous" => Some(Input::Previous),
            "next" => Some(Input::Next),
            "select" => Some(Input::Select),
            "back" => Some(Input::Back),
            _ => None,
        }
    }
}

/// Tout ce qui peut réveiller la boucle principale.
pub enum Event {
    Input(Input),
    MpvClosed,
}

struct App {
    mpv: Mpv,
    videos_dir: PathBuf,
    menu: Menu,
    active: Option<Category>,
    /// Message affiché à la place d'une vidéo (ex : dossier vide).
    notice: Option<(String, String)>,
}

impl App {
    fn handle(&mut self, input: Input) -> io::Result<()> {
        match input {
            // Tourner le bouton pendant la lecture ouvre d'abord le menu.
            Input::Previous | Input::Next if !self.menu.open => self.menu.show(),
            Input::Previous => self.menu.move_by(-1),
            Input::Next => self.menu.move_by(1),
            Input::Select if !self.menu.open => self.menu.show(),
            Input::Select => {
                self.activate(self.menu.selected())?;
                self.menu.hide();
            }
            Input::Back if self.active.is_some() => self.menu.hide(),
            Input::Back => {}
        }
        Ok(())
    }

    fn activate(&mut self, category: Category) -> io::Result<()> {
        self.active = Some(category);
        self.notice = None;

        let Some(folder) = category.video_folder() else {
            return self.mpv.stop(); // Clock et Weather s'affichent sur fond noir
        };
        let dir = self.videos_dir.join(folder);
        let files = list_videos(&dir);
        if files.is_empty() {
            self.notice = Some((category.label().to_string(), format!("Aucune vidéo dans {}", dir.display())));
            return self.mpv.stop();
        }
        self.mpv.play(&files)
    }

    /// Le menu se referme tout seul s'il reste ouvert sans action.
    fn tick(&mut self) {
        if self.active.is_some() && self.menu.expired() {
            self.menu.hide();
        }
    }

    /// Redessine les calques à partir de l'état (appelé après chaque événement).
    fn render(&mut self) -> io::Result<()> {
        if self.menu.open {
            self.mpv.set_overlay(MENU_OVERLAY, &overlay::menu(self.menu.selected_index()))?;
        } else {
            self.mpv.remove_overlay(MENU_OVERLAY)?;
        }

        let screen = match (self.active, &self.notice) {
            (_, Some((title, detail))) => Some(overlay::message(title, detail)),
            (Some(Category::Clock), _) => Some(overlay::clock(Local::now())),
            (Some(Category::Weather), _) => Some(overlay::message("Weather", "Bientôt disponible")),
            _ => None,
        };
        match screen {
            Some(ass) => self.mpv.set_overlay(SCREEN_OVERLAY, &ass),
            None => self.mpv.remove_overlay(SCREEN_OVERLAY),
        }
    }
}

/// Fichiers vidéo du dossier, triés par nom (vide si le dossier n'existe pas).
fn list_videos(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && is_video(path))
        .collect();
    files.sort();
    files
}

fn is_video(path: &Path) -> bool {
    let hidden = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'));
    let extension = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase);
    !hidden && extension.is_some_and(|e| VIDEO_EXTENSIONS.contains(&e.as_str()))
}

/// Délai jusqu'à la prochaine seconde pile, pour que l'horloge change au bon moment.
fn until_next_second() -> Duration {
    let millis = Local::now().nanosecond() / 1_000_000;
    Duration::from_millis(1000 - u64::from(millis.min(999)))
}

fn main() -> io::Result<()> {
    let videos_dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "videos".to_string()));
    println!("Dossier des vidéos : {}", videos_dir.display());

    let (events, inbox) = mpsc::channel();
    let mpv = Mpv::launch(events)?;
    let mut app = App { mpv, videos_dir, menu: Menu::new(), active: None, notice: None };
    app.render()?;

    loop {
        match inbox.recv_timeout(until_next_second()) {
            Ok(Event::Input(input)) => app.handle(input)?,
            Ok(Event::MpvClosed) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        app.tick();
        app.render()?;
    }

    println!("mpv s'est arrêté, fin du programme.");
    Ok(())
}
