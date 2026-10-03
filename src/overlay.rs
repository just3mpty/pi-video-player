//! Rendu des calques en texte ASS (le format de sous-titres que mpv sait dessiner).
//! Rappels : les tags sont entre accolades, `\N` = retour à la ligne,
//! les couleurs sont en BGR (`&HBBGGRR&`) et l'alpha va de 00 (opaque) à FF (invisible).

use chrono::{DateTime, Datelike, Local, Timelike};

use crate::menu::Category;

/// Palette du projet, en BGR pour l'ASS : #0F0F0F, #DCD7D0 et #4C63D2.
const BLACK: &str = "&H0F0F0F&";
const WHITE: &str = "&HD0D7DC&";
const BLUE: &str = "&HD2634C&";
/// Transparence des textes secondaires (00 = opaque, FF = invisible).
const DIM: &str = "&H60&";

/// Dégradé noir sur la gauche de l'écran, pour que le texte reste lisible quelle que soit
/// la vidéo derrière. L'ASS ne sait pas faire de dégradé : on empile de fines bandes
/// verticales de plus en plus transparentes.
const SHADE_WIDTH: u32 = 640;
const SHADE_STRIPS: u32 = 64;

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
    // Fine ligne verticale qui sépare la liste du reste de l'écran.
    lines.push(format!(r"{{\an7\pos(486,64)\bord0\shad0\1c{WHITE}\1a&HA0&\p1}}{}", rect(2, 592)));
    lines.push(format!(r"{{\an4\pos({LEFT},{TITLE_Y})\bord0\shad0\fn{FONT}\b1\fs66\1c{WHITE}}}MOODS"));

    let mut y = FIRST_ITEM_Y;
    for (i, category) in Category::ALL.iter().enumerate() {
        if i == selected {
            y += SELECTED_GAP;
            lines.push(format!(r"{{\an4\pos({LEFT},{y})\bord0\shad0\fn{FONT}\fs46\1c{WHITE}}}{}", label(*category)));
            lines.push(format!(r"{{\an4\pos(526,{y})\bord0\shad0\fn{FONT}\fs30\1c{BLUE}}}<"));
            y += SELECTED_GAP;
        } else {
            lines.push(format!(r"{{\an4\pos({LEFT},{y})\bord0\shad0\fn{FONT}\fs29\1c{WHITE}\1a{DIM}}}{}", label(*category)));
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
            format!(r"{{\an7\pos({x},0)\bord0\shad0\1c{BLACK}\1a&H{alpha:02X}&\p1}}{}", rect(width, 720))
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

/// Horloge : l'heure en grand, une barre bleue qui se remplit au fil des secondes,
/// puis la date en petites capitales espacées.
pub fn clock(now: DateTime<Local>, menu_open: bool) -> String {
    let x = center_x(menu_open);
    let date = format!(
        "{} {} {}",
        DAYS[now.weekday().num_days_from_monday() as usize],
        now.day(),
        MONTHS[now.month0() as usize],
    )
    .to_uppercase();

    let track = 480;
    let progress = track * (now.second() + 1) / 60;
    let left = x - track / 2;
    [
        format!(r"{{\an2\pos({x},400)\bord0\shad0\fn{FONT}\b1\fs200\fsp-4\1c{WHITE}}}{}", now.format("%H:%M")),
        format!(r"{{\an7\pos({left},420)\bord0\shad0\1c{WHITE}\1a&HD0&\p1}}{}", rect(track, 4)),
        format!(r"{{\an7\pos({left},420)\bord0\shad0\1c{BLUE}\p1}}{}", rect(progress, 4)),
        format!(r"{{\an8\pos({x},448)\bord0\shad0\fn{FONT}\fs30\fsp8\1c{WHITE}\1a{DIM}}}{date}"),
    ]
    .join("\n")
}

/// Même mise en page que l'horloge, en plus petit : titre, trait bleu, détail.
pub fn message(title: &str, detail: &str, menu_open: bool) -> String {
    let x = center_x(menu_open);
    [
        format!(r"{{\an2\pos({x},360)\bord0\shad0\fn{FONT}\b1\fs64\1c{WHITE}}}{}", escape(title)),
        format!(r"{{\an7\pos({},380)\bord0\shad0\1c{BLUE}\p1}}{}", x - 32, rect(64, 4)),
        format!(r"{{\an8\pos({x},404)\bord0\shad0\fn{FONT}\fs30\1c{WHITE}\1a{DIM}}}{}", escape(detail)),
    ]
    .join("\n")
}

/// Rectangle `w` × `h` en mode dessin ASS (`m` = déplacer, `l` = tracer une ligne).
fn rect(w: u32, h: u32) -> String {
    format!(r"m 0 0 l {w} 0 l {w} {h} l 0 {h}{{\p0}}")
}

/// Neutralise les caractères que l'ASS interpréterait comme des tags.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\u{2060}").replace('{', r"\{").replace('}', r"\}")
}
