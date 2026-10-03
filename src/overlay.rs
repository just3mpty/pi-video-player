//! Rendu des calques en texte ASS (le format de sous-titres que mpv sait dessiner).
//! Rappels : les tags sont entre accolades, `\N` = retour à la ligne,
//! les couleurs sont en BGR (`&HBBGGRR&`) et l'alpha va de 00 (opaque) à FF (invisible).

use chrono::{DateTime, Datelike, Local};

use crate::menu::Category;

/// Dégradé noir sur la gauche de l'écran, pour que le texte reste lisible quelle que soit
/// la vidéo derrière. L'ASS ne sait pas faire de dégradé : on empile de fines bandes
/// verticales de plus en plus transparentes.
const SHADE_WIDTH: u32 = 640;
const SHADE_STRIPS: u32 = 64;

/// Fine ligne verticale qui sépare la liste du reste de l'écran.
const MENU_DIVIDER: &str = r"{\an7\pos(486,64)\bord0\shad0\1c&HFFFFFF&\1a&HA0&\p1}m 0 0 l 2 0 l 2 592 l 0 592{\p0}";

const FONT: &str = "Inter";
const LEFT: u32 = 96;
const TITLE_Y: u32 = 134;
const FIRST_ITEM_Y: u32 = 222;
const ITEM_STEP: u32 = 46;
/// Écart supplémentaire au-dessus et en dessous de l'élément sélectionné, plus gros.
const SELECTED_GAP: u32 = 10;

/// Milieu de la zone à droite du séparateur, pour centrer l'horloge et les messages
/// quand le menu est affiché ; sinon ils sont au centre de l'écran.
const RIGHT_CENTER_X: u32 = 883;
const SCREEN_CENTER_X: u32 = 640;

const DAYS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
const MONTHS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin",
    "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

/// Menu vertical sur la gauche : titre, liste des catégories et repère `<` sur la sélection.
/// Chaque ligne est un événement ASS positionné à la main pour coller à la maquette.
pub fn menu(selected: usize) -> String {
    let mut lines = shade();
    lines.push(MENU_DIVIDER.to_string());
    lines.push(format!(r"{{\an4\pos({LEFT},{TITLE_Y})\bord0\shad0\fn{FONT}\b1\fs66\1c&HDDE3E8&}}MOODS"));

    let mut y = FIRST_ITEM_Y;
    for (i, category) in Category::ALL.iter().enumerate() {
        if i == selected {
            y += SELECTED_GAP;
            lines.push(format!(r"{{\an4\pos({LEFT},{y})\bord0\shad0\fn{FONT}\fs46\1c&HFFFFFF&}}{}", label(*category)));
            lines.push(format!(r"{{\an4\pos(526,{y})\bord0\shad0\fn{FONT}\fs30\1c&HFFFFFF&}}<"));
            y += SELECTED_GAP;
        } else {
            lines.push(format!(r"{{\an4\pos({LEFT},{y})\bord0\shad0\fn{FONT}\fs29\1c&HFFFFFF&\1a&H60&}}{}", label(*category)));
        }
        y += ITEM_STEP;
    }
    lines.join("\n")
}

fn shade() -> Vec<String> {
    let width = SHADE_WIDTH / SHADE_STRIPS;
    (0..SHADE_STRIPS)
        .map(|i| {
            // Opacité de ~80 % à gauche jusqu'à 0 : sombre longtemps, puis s'efface en douceur.
            let t = f64::from(i) / f64::from(SHADE_STRIPS);
            let alpha = (f64::from(0x30) + f64::from(0xFF - 0x30) * t * t) as u8;
            let x = i * width;
            format!(r"{{\an7\pos({x},0)\bord0\shad0\1c&H000000&\1a&H{alpha:02X}&\p1}}m 0 0 l {width} 0 l {width} 720 l 0 720{{\p0}}")
        })
        .collect()
}

/// « Lofi (1) » : le nombre de vidéos, sauf pour les écrans sans vidéo (Clock, Weather).
fn label(category: Category) -> String {
    match category.videos() {
        Some(videos) => format!("{} ({})", category.label(), videos.len()),
        None => category.label().to_string(),
    }
}

fn center_x(menu_open: bool) -> u32 {
    if menu_open { RIGHT_CENTER_X } else { SCREEN_CENTER_X }
}

pub fn clock(now: DateTime<Local>, menu_open: bool) -> String {
    let x = center_x(menu_open);
    let date = format!(
        "{} {} {}",
        DAYS[now.weekday().num_days_from_monday() as usize],
        now.day(),
        MONTHS[now.month0() as usize],
    );
    format!(r"{{\an5\pos({x},360)\bord3\shad0\fs130}}{}\N{{\fs36\1c&HCCCCCC&}}{date}", now.format("%H:%M:%S"))
}

pub fn message(title: &str, detail: &str, menu_open: bool) -> String {
    let x = center_x(menu_open);
    format!(r"{{\an5\pos({x},360)\bord2\shad0\fs64}}{}\N{{\fs30\1c&HCCCCCC&}}{}", escape(title), escape(detail))
}

/// Neutralise les caractères que l'ASS interpréterait comme des tags.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\u{2060}").replace('{', r"\{").replace('}', r"\}")
}
