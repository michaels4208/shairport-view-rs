use fltk::{group::{Flex, Group}, frame::Frame, image::{SharedImage, PngImage}, prelude::*,
           window::Window, enums::{Align, Color, FrameType}};
use std::error::Error;

const INIT_WINDOW_W: i32 = 1200;
const INIT_WINDOW_H: i32 = 900;
const DEFAULT_ART_BYTES: &[u8; 29187] = include_bytes!("resources/img/no-art.png");

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

/// Contains all user interface items that can be interacted with
pub struct UI {
    art: Frame,
    title: Frame,
    artist: Frame,
}

impl UI {
    /// Set the title text in the GUI
    pub fn set_title(&mut self, title: &str) {
        self.title.set_label(title);
    }

    /// Set the artist text in the GUI
    pub fn set_artist(&mut self, artist: &str) {
        self.artist.set_label(artist);
    }

    /// Set the album text in the GUI (currently not displayed)
    pub fn set_album(&mut self, _: &str) {}  // UI currently doesn't have an Album field

    /// Set the album art
    pub fn set_art<T: ImageExt>(&mut self, art: T) {
        // art.scale(self.art.w(), self.art.h(), true, true);
        // self.art.set_image(Some(art));
        self.art.set_image_scaled(Some(art));  // Can I just do this instead?  Try it.
    }
}

/// Function to create column and row Flex Widgets for holding pad, item, pad col and row
/// This allows the pads to stay fixed, but the center to scale
pub fn make_window() -> (Window, UI)
{
    let mut main_win = Window::default()
        .with_size(INIT_WINDOW_W, INIT_WINDOW_H)
        .center_screen()
        .with_label("shairport-view-rs");
    main_win.make_resizable(true);

    let ui = make_win_pad_flexes(&main_win);

    main_win.size_range(640, 480, 1600, 1200);
    main_win.end();
    (main_win, ui)
}

fn make_win_pad_flexes<G>(win: &G) -> UI
    where G: GroupExt,
{
    let pad_size = 20;

    // Make flex column to hold top pad, main row, and bottom pad
    let mut win_col = Flex::default_fill().column();
    let mut top_pad = Frame::default_fill();

    // Make a flex row inside the flex column to hold left pad, main widget, right pad
    let mut win_row = Flex::default_fill().row();
    let mut left_pad = Frame::default_fill();
    let ui = make_main_layout(&win_row);  // Content Frame
    let mut right_pad = Frame::default_fill();
    win_row.set_size(&left_pad, pad_size);  // Calling Flex::set_size makes a thing a fixed size
    win_row.set_size(&right_pad, pad_size);
    win_row.make_resizable(true);
    win_row.set_pad(0);
    win_row.set_margin(0);
    win_row.end();

    let mut bot_pad = Frame::default_fill();

    debug_draw_frame_boundary(&mut top_pad, Color::Magenta);
    debug_draw_frame_boundary(&mut left_pad, Color::Magenta);
    debug_draw_frame_boundary(&mut right_pad, Color::Magenta);
    debug_draw_frame_boundary(&mut bot_pad, Color::Magenta);

    win_col.set_size(&top_pad, pad_size);
    win_col.set_size(&bot_pad, pad_size);
    win_col.resizable(&win_row);
    win_col.make_resizable(true);
    win_col.set_pad(0);
    win_col.set_margin(0);
    win_col.end();
    win.resizable(&win_col);
    ui
}

// Makes the Group that defines the area inside the Window padding
// Uses a group instead of a column, so that all components inside scale together
fn make_main_layout<G>(parent: &G) -> UI
    where G: GroupExt,
{
    let main_group = Group::default_fill();

    let mut top_area = Frame::default()
        .with_size(main_group.w(), main_group.h() * 2 / 10);

    //let mut art_title_area = Frame::default()
    //    .with_size(main_group.w(), main_group.h() * 6 / 10)
    //    .below_of(&top_area, 0);
    let mut art_title_area = Group::default()
        .with_size(main_group.w(), main_group.h() * 6 / 10)
        .below_of(&top_area, 0);
    let ui = make_art_title_layout(&mut art_title_area);

    let mut bot_area = Frame::default()
        .with_size(main_group.w(), main_group.h() * 2 / 10)
        .below_of(&art_title_area, 0);

    debug_draw_frame_boundary(&mut top_area, Color::Blue);
    debug_draw_frame_boundary(&mut bot_area, Color::Blue);

    main_group.end();
    parent.resizable(&main_group);
    ui
}

fn make_art_title_layout<G>(art_title_area: &mut G) -> UI
    where G: WidgetBase + GroupExt,
{
    art_title_area.end();
    let (art_idx, title_idx, artist_idx) = (0, 1, 2);

    let mut art = Frame::default_fill();
    let art_image = SharedImage::from_image(PngImage::from_data(DEFAULT_ART_BYTES).unwrap()).unwrap();
    //// frame.set_image(None::<SharedImage>);  // To remove image
    art.set_image(Some(art_image));
    art_title_area.insert(&art, art_idx);

    let mut title = Frame::default_fill()
        .with_pos(0, 0)
        .with_align(Align::BottomLeft | Align::Inside);
    title.set_label("");
    title.set_label_size(45);
    art_title_area.insert(&title, title_idx);

    let mut artist = Frame::default_fill()
        .with_align(Align::TopLeft | Align::Inside);
    artist.set_label("");
    artist.set_label_size(45);
    art_title_area.insert(&artist, artist_idx);

    art_title_area.resize_callback(move |grp, x, y, w, h| {
        let art_side_w = w * 4 / 10;
        let vert_spacer = 40;

        // Art is square, and no bigger than either the height or 40% of the width of this area
        let art_size = std::cmp::min(h, art_side_w);
        let art_x = x + (w * 4 / 10 - art_size);  // Right-justify 40% into area
        let art_y = y + (h - art_size) / 2;  // Center vertically in area

        let text_x = x + art_side_w + vert_spacer;  // Text is right of the art area, plus a spacer
        let text_w = w - art_side_w - vert_spacer;
        let text_h = art_size * 2 / 10;
        let title_y = art_y + art_size * 7 / 20;  // Title starts % down from top of art
        let artist_y = art_y + art_size * 13 / 20;  // Artist starts 70% down from top of art

        if let Some(mut art_frame) = grp.child(art_idx) {
            art_frame.set_size(art_size, art_size);
            art_frame.set_pos(art_x, art_y);
            unsafe {
                // image_mut is only unsafe when the frame no longer exists
                // But this callback should only be called when the frame does exist, so this code should be fine.
                if let Some(art_image) = art_frame.image_mut() {
                    art_image.scale(art_size, art_size, true, true)
                }
            }
            art_frame.redraw();
        };

        if let Some(mut title_frame) = grp.child(title_idx) {
            title_frame.set_size(text_w, text_h);
            title_frame.set_pos(text_x, title_y);
        }

        if let Some(mut artist_frame) = grp.child(artist_idx) {
            artist_frame.set_size(text_w, text_h);
            artist_frame.set_pos(text_x, artist_y);
        }
    });

    // Add colored boarders around all frames for easier layout dev/debug
    debug_draw_frame_boundary(&mut art, Color::Green);
    debug_draw_frame_boundary(&mut title, Color::Black);
    debug_draw_frame_boundary(&mut artist, Color::Black);

    UI{art, title, artist}
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn debug_draw_frame_boundary(frame: &mut Frame, color: Color) {
    // Draw frame boundaries when running `cargo build` or `cargo run`
    frame.set_color(color);
    frame.set_frame(FrameType::BorderFrame);
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn debug_draw_frame_boundary(_frame: &mut Frame, _color: Color) {
    // Don't draw frame boundaries when running `cargo build --release` or `cargo run --release`
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn debug_message(s: &str) {
    // Print debug messages only when running in non --release mode
    println!("{}", s);
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn debug_message(_s: &str) {
}
