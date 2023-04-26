use fltk::{self, prelude::*};
use std::{thread, error::Error};
use shairport_view_rs::{gui, meta_gen::{self, Metadata}};


fn main() -> Result<(), Box<dyn Error>> {
    let app = fltk::app::App::default();
    let (mut main_win, mut ui) = gui::make_window();
    main_win.show();

    // let ui_thread = thread::spawn(move || { gui::sample_metadata(&mut ui) });
    // while app.wait() {
    //     main_win.redraw();
    //     if ui_thread.is_finished() { break }
    // }



    let meta_handler = move |metadata: Metadata| -> Result<(), Box<dyn Error>> {
        match metadata {
            Metadata::Track(track) => ui.set_title(&track),
            Metadata::Artist(artist) => ui.set_artist(&artist),
            Metadata::Album(album) => ui.set_album(&album),
            Metadata::Art(art) => ui.set_art(art),
        };
        fltk::app::awake();
        Ok(())
    };

    let metadata_thread = thread::spawn(move ||
        meta_gen::parse_metadata(Some("/tmp/shairport-sync-metadata"), meta_handler));

    while app.wait() {
        main_win.redraw();
        if metadata_thread.is_finished() { break }
    }

    Ok(())
}