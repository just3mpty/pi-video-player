//! Rendu des calques en texte ASS (le format de sous-titres que mpv sait dessiner).
//! Rappels : les tags sont entre accolades, `\N` = retour à la ligne,
//! les couleurs sont en BGR (`&HBBGGRR&`) et l'alpha va de 00 (opaque) à FF (invisible).

use chrono::{DateTime, Datelike, Local};

use crate::menu::Category;

/// Encart noir semi-transparent aux coins arrondis, à gauche, centré verticalement :
/// 240 × 260 en (20, 230). Dessiné avec des lignes (`l`) et des courbes de Bézier (`b`).
const MENU_PANEL: &str = concat!(
    r"{\an7\pos(20,230)\bord0\shad0\1c&H000000&\1a&H40&\p1}",
    "m 18 0 l 222 0 b 232 0 240 8 240 18 l 240 242 b 240 252 232 260 222 260 ",
    "l 18 260 b 8 260 0 252 0 242 l 0 18 b 0 8 8 0 18 0",
    r"{\p0}",
);

const DAYS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
const MONTHS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin",
    "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];

/// Menu vertical dans l'encart de gauche. Toutes les lignes ont la même taille
/// pour que la liste ne bouge pas quand la sélection change.
pub fn menu(selected: usize) -> String {
    let mut text = String::from(r"{\an7\pos(44,250)\bord0\shad0\fs22\1c&H999999&}MOOD\N");
    for (i, category) in Category::ALL.iter().enumerate() {
        if i == selected {
            text += r"{\fs34\b1\1c&HFFFF00&\1a&H00&}";
        } else {
            text += r"{\fs34\b0\1c&HFFFFFF&\1a&H40&}";
        }
        text += category.label();
        text += r"\N";
    }
    format!("{MENU_PANEL}\n{text}")
}

pub fn clock(now: DateTime<Local>) -> String {
    let date = format!(
        "{} {} {}",
        DAYS[now.weekday().num_days_from_monday() as usize],
        now.day(),
        MONTHS[now.month0() as usize],
    );
    // Taille limitée pour que l'heure, centrée, ne passe pas sous l'encart du menu.
    format!(r"{{\an5\bord3\shad0\fs130}}{}\N{{\fs36\1c&HCCCCCC&}}{date}", now.format("%H:%M:%S"))
}

pub fn message(title: &str, detail: &str) -> String {
    format!(r"{{\an5\bord2\shad0\fs64}}{}\N{{\fs30\1c&HCCCCCC&}}{}", escape(title), escape(detail))
}

/// Neutralise les caractères que l'ASS interpréterait comme des tags.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\u{2060}").replace('{', r"\{").replace('}', r"\}")
}
