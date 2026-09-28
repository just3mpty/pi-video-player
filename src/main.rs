use std::env;
use std::process::Command;

fn main() {
    let video = env::args()
        .nth(1)
        .expect("Usage : pi-video-player <chemin_de_la_video>");

    println!("Lancement de : {video}");

    let status = Command::new("mpv")
        .args([
            "--vo=gpu",
            "--gpu-context=drm",
            "--hwdec=auto",
            "--fs",
            "--loop=inf",
            &video,
        ])
        .status()
        .expect("Impossible de lancer mpv (est-il installé ?)");

    println!("mpv s'est terminé avec : {status}");
}