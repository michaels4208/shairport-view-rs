use fltk::{app, group::{Flex, Group}, frame::Frame, image::SharedImage, prelude::*, window::Window,
    enums::{Align, Color, FrameType}};
use std::error::Error;
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

const INIT_WINDOW_W: i32 = 1200;
const INIT_WINDOW_H: i32 = 900;
const SHOW_LAYOUT: bool = false;  // Display frame boundaries

#[derive(Debug, Clone)]
pub enum UIError {
    NoSource,
}

impl std::fmt::Display for UIError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UIError::NoSource => write!(f, "No Metadata source to UI")
        }
    }
}

impl Error for UIError {}

struct UI {
    art: Frame,
    title: Frame,
    artist: Frame,
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = app::App::default();
    let (mut main_win, mut ui) = make_window();
    main_win.show();

    //let ui_thread = thread::spawn(move || { poll_for_metadata(ui) });
    let ui_thread = thread::spawn(move || {
        debug_message("poll_for_metadata started");
        app::sleep(2.);

        debug_message(&format!("poll_for_metadata output1: title={}, artist={}", ui.title.label(), ui.artist.label()));
        ui.title.set_label("Like an Armenian");
        ui.artist.set_label("Lana Del Mar");
        debug_message(&format!("poll_for_metadata after output1: title={}, artist={}", ui.title.label(), ui.artist.label()));
        app::awake();

        debug_message("poll_for_metadata sleep before output2");
        app::sleep(2.);

        debug_message("poll_for_metadata output2");
        ui.title.set_label("Here Conmigo");
        ui.artist.set_label("C4vrch35");
        app::awake();

        debug_message("poll_for_metadata sleep after output2");
        app::sleep(2.);
        debug_message("poll_for_metadata done");
    });

    app.run().unwrap();

    ui_thread.join().unwrap();

    Ok(())
}

// Function to create column and row Flex Widgets for holding pad, item, pad col and row
// This allows the pads to stay fixed, but the center to scale
fn make_window() -> (Window, UI)
{
    let mut main_win = Window::default()
        .with_size(INIT_WINDOW_W, INIT_WINDOW_H)
        .center_screen()
        .with_label("shairport-view-rs");
    main_win.make_resizable(true);

    let mut title = Frame::default().with_size(400, 100).with_pos(100, 0).with_align(Align::Inside).with_label("Song Title");
    let artist = Frame::default().with_size(400, 100).with_pos(100,100).with_align(Align::Inside);
    let art = Frame::default().with_pos(100,200).with_align(Align::Inside);

    title.set_label_size(60);
    title.set_label_color(Color::Black);

    let ui = UI{art, title, artist};

    main_win.size_range(640, 480, 1600, 1200);
    main_win.end();
    (main_win, ui)
}


#[cfg(debug_assertions)]
fn debug_draw_frame_boundary(frame: &mut Frame, color: Color) {
    frame.set_color(color);
    frame.set_frame(FrameType::BorderFrame);
}

#[cfg(not(debug_assertions))]
fn debug_draw_frame_boundary(frame: &mut Frame, color: Color) {
}

#[cfg(debug_assertions)]
fn debug_message(s: &str) {
    println!("{}", s);
}

#[cfg(not(debug_assertions))]
fn debug_message(s: &str) {
}