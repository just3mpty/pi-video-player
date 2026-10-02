//! Pilotage de mpv : lancement du processus et dialogue via son socket IPC (JSON).
//! Doc du protocole : https://mpv.io/manual/stable/#json-ipc

use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::{Event, Input};

const INPUT_CONF: &str = include_str!("input.conf");

pub struct Mpv {
    process: Child,
    socket: UnixStream,
}

impl Mpv {
    /// Lance mpv (fenêtre noire en attente) et transmet ses événements sur `events`.
    pub fn launch(events: Sender<Event>) -> io::Result<Mpv> {
        let socket_path = env::temp_dir().join("pi-video-player.sock");
        let input_conf = env::temp_dir().join("pi-video-player-input.conf");
        fs::write(&input_conf, INPUT_CONF)?;
        // Un socket laissé par un lancement précédent ferait croire que mpv est prêt.
        let _ = fs::remove_file(&socket_path);

        let mut command = Command::new("mpv");
        command
            .args([
                "--no-config",
                "--idle=yes",
                "--force-window=yes",
                "--fs",
                "--no-audio",
                "--loop-playlist=inf",
                "--hwdec=auto",
                "--osc=no",
                "--osd-bar=no",
                "--cursor-autohide=always",
                "--input-default-bindings=no",
                "--quiet",
            ])
            .arg(format!("--input-conf={}", input_conf.display()))
            .arg(format!("--input-ipc-server={}", socket_path.display()));
        // Sur le Pi, pas de serveur graphique : mpv affiche directement via DRM/KMS.
        if cfg!(target_os = "linux") {
            command.args(["--vo=gpu", "--gpu-context=drm"]);
        }

        let process = command.spawn().map_err(|e| {
            io::Error::new(e.kind(), format!("impossible de lancer mpv (est-il installé ?) : {e}"))
        })?;
        let socket = connect(&socket_path, Duration::from_secs(10))?;

        let reader = socket.try_clone()?;
        thread::spawn(move || read_events(reader, events));

        Ok(Mpv { process, socket })
    }

    /// Joue les fichiers en boucle, à la place de ce qui passait.
    pub fn play(&mut self, files: &[PathBuf]) -> io::Result<()> {
        for (i, file) in files.iter().enumerate() {
            let mode = if i == 0 { "replace" } else { "append" };
            self.command(json!(["loadfile", file.to_string_lossy(), mode]))?;
        }
        Ok(())
    }

    /// Arrête la lecture (écran noir).
    pub fn stop(&mut self) -> io::Result<()> {
        self.command(json!(["stop"]))
    }

    /// Affiche (ou remplace) un calque de texte ASS identifié par `id`.
    /// Coordonnées : 720 de haut, largeur déduite du ratio de l'écran.
    pub fn set_overlay(&mut self, id: u32, ass: &str) -> io::Result<()> {
        self.command(json!({
            "name": "osd-overlay",
            "id": id,
            "format": "ass-events",
            "data": ass,
            "res_x": 0,
            "res_y": 720,
        }))
    }

    pub fn remove_overlay(&mut self, id: u32) -> io::Result<()> {
        self.command(json!({ "name": "osd-overlay", "id": id, "format": "none", "data": "" }))
    }

    fn command(&mut self, command: Value) -> io::Result<()> {
        let mut line = json!({ "command": command }).to_string();
        line.push('\n');
        self.socket.write_all(line.as_bytes())
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        let _ = self.command(json!(["quit"]));
        let _ = self.process.wait();
    }
}

/// mpv met un peu de temps à créer son socket : on réessaie jusqu'au délai.
fn connect(path: &Path, timeout: Duration) -> io::Result<UnixStream> {
    let start = Instant::now();
    loop {
        match UnixStream::connect(path) {
            Ok(socket) => return Ok(socket),
            Err(e) if start.elapsed() > timeout => {
                return Err(io::Error::new(e.kind(), format!("mpv ne répond pas sur {} : {e}", path.display())));
            }
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// Tourne dans son propre thread : lit les messages de mpv ligne par ligne.
fn read_events(socket: UnixStream, events: Sender<Event>) {
    for line in BufReader::new(socket).lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };

        if message["event"] == "client-message" && message["args"][0] == "pvp" {
            let input = message["args"][1].as_str().and_then(Input::parse);
            if let Some(input) = input
                && events.send(Event::Input(input)).is_err()
            {
                return; // le programme principal est terminé
            }
        } else if message["error"].as_str().is_some_and(|e| e != "success") {
            eprintln!("mpv a refusé une commande : {line}");
        }
    }
    // Socket fermé : mpv s'est arrêté (touche q, fenêtre fermée, crash…).
    let _ = events.send(Event::MpvClosed);
}
