mod menu;
mod mpv;
mod overlay;
mod gpio;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

        let Some(names) = category.videos() else {
            return self.mpv.stop(); // Clock et Weather s'affichent sur fond noir
        };
        let paths: Vec<PathBuf> = names.iter().map(|name| self.videos_dir.join(name)).collect();
        let (files, missing): (Vec<PathBuf>, Vec<PathBuf>) = paths.into_iter().partition(|p| p.is_file());
        for path in &missing {
            eprintln!("Vidéo introuvable : {}", path.display());
        }
        if files.is_empty() {
            self.notice = Some((category.label().to_string(), format!("Vidéo introuvable dans {}", self.videos_dir.display())));
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

        let open = self.menu.open;
        let screen = match (self.active, &self.notice) {
            (_, Some((title, detail))) => Some(overlay::message(title, detail, open)),
            (Some(Category::Clock), _) => Some(overlay::clock(Local::now(), open)),
            (Some(Category::Weather), _) => Some(overlay::message("Weather", "Bientôt disponible", open)),
            _ => None,
        };
        match screen {
            Some(ass) => self.mpv.set_overlay(SCREEN_OVERLAY, &ass),
            None => self.mpv.remove_overlay(SCREEN_OVERLAY),
        }
    }
}

/// Une vidéo au hasard dans le dossier, pour l'ambiance au démarrage.
/// Pas besoin d'une crate de hasard pour un seul tirage : les nanosecondes de l'heure suffisent.
fn random_video(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    let videos: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && is_video(path))
        .collect();
    if videos.is_empty() {
        return None;
    }
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.subsec_nanos() as usize;
    Some(videos[nanos % videos.len()].clone())
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
    // Par défaut ~/media, le même chemin sur le PC de dev et sur le Pi.
    let videos_dir = match env::args().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env::var("HOME").unwrap_or_default()).join("media"),
    };
    println!("Dossier des vidéos : {}", videos_dir.display());

    let (events, inbox) = mpsc::channel();

    {
    let events = events.clone();
    std::thread::spawn(move || {
        if let Err(e) = gpio::listen(events) {
            eprintln!("Erreur GPIO : {e}");
        }
    });
}

    let mpv = Mpv::launch(events)?;
    let mut app = App { mpv, videos_dir, menu: Menu::new(), active: None, notice: None };
    match random_video(&app.videos_dir) {
        Some(video) => app.mpv.play(&[video])?,
        None => eprintln!("Aucune vidéo dans {}, démarrage sur fond noir.", app.videos_dir.display()),
    }
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
