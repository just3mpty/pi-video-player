use std::time::{Duration, Instant};

const MENU_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Category {
    Cyberpunk,
    Lofi,
    Weather,
    Clock,
}

impl Category {
    pub const ALL: [Category; 4] = [Category::Cyberpunk, Category::Lofi, Category::Weather, Category::Clock];

    pub fn label(self) -> &'static str {
        match self {
            Category::Cyberpunk => "Cyberpunk",
            Category::Lofi => "Lofi",
            Category::Weather => "Weather",
            Category::Clock => "Clock",
        }
    }

    /// Vidéos jouées en boucle pour cette catégorie, relatives au dossier des vidéos.
    /// `None` = catégorie sans vidéo (affichage sur fond noir).
    pub fn videos(self) -> Option<&'static [&'static str]> {
        match self {
            Category::Cyberpunk => Some(&["test.mp4"]),
            Category::Lofi => Some(&["une nuit au clair de lune (24fps).mp4"]),
            Category::Weather | Category::Clock => None,
        }
    }
}

pub struct Menu {
    pub open: bool,
    selected: usize,
    last_activity: Instant,
}

impl Menu {
    pub fn new() -> Menu {
        Menu { open: true, selected: 0, last_activity: Instant::now() }
    }

    pub fn selected(&self) -> Category {
        Category::ALL[self.selected]
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn show(&mut self) {
        self.open = true;
        self.last_activity = Instant::now();
    }

    pub fn hide(&mut self) {
        self.open = false;
    }

    pub fn move_by(&mut self, delta: isize) {
        let len = Category::ALL.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
        self.last_activity = Instant::now();
    }

    pub fn expired(&self) -> bool {
        self.open && self.last_activity.elapsed() >= MENU_TIMEOUT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_around() {
        let mut menu = Menu::new();
        menu.move_by(-1);
        assert_eq!(menu.selected(), Category::Clock);
        menu.move_by(1);
        assert_eq!(menu.selected(), Category::Cyberpunk);
    }
}
