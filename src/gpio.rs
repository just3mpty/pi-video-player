use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use gpiocdev::line::{Bias, EdgeDetection, EdgeKind, Value};
use gpiocdev::Request;

use crate::{Event, Input};

const CLK: u32 = 22;
const DT: u32 = 23;
const SW: u32 = 27;

const TABLE: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

/// Écoute la molette en continu et pousse les Input correspondants dans le canal principal.
pub fn listen(events: Sender<Event>) -> std::io::Result<()> {
    let req = Request::builder()
        .on_chip("/dev/gpiochip0")
        .with_consumer("pi-video-player")
        .with_lines(&[CLK, DT, SW])
        .as_input()
        .with_bias(Bias::PullUp)
        .with_edge_detection(EdgeDetection::BothEdges)
        .with_debounce_period(Duration::from_micros(500))
        .request()
        .map_err(|e| std::io::Error::other(e))?;

    let read = |offset: u32| -> std::io::Result<u8> {
        let v = req.value(offset).map_err(std::io::Error::other)?;
        Ok(if v == Value::Active { 1 } else { 0 })
    };

    let mut prev = (read(CLK)? << 1) | read(DT)?;
    let mut acc: i8 = 0;
    let mut last_click = Instant::now() - Duration::from_secs(1);

    loop {
        let ev = req.read_edge_event().map_err(std::io::Error::other)?;
        match ev.offset {
            CLK | DT => {
                let new = (read(CLK)? << 1) | read(DT)?;
                acc += TABLE[((prev << 2) | new) as usize];
                prev = new;

                if acc >= 4 {
                    let _ = events.send(Event::Input(Input::Next));
                    acc = 0;
                } else if acc <= -4 {
                    let _ = events.send(Event::Input(Input::Previous));
                    acc = 0;
                }
                if new == 0b11 {
                    acc = 0;
                }
            }
            SW => {
                if ev.kind == EdgeKind::Falling && last_click.elapsed() > Duration::from_millis(200) {
                    let _ = events.send(Event::Input(Input::Select));
                    last_click = Instant::now();
                }
            }
            _ => {}
        }
    }
}