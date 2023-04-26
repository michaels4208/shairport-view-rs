use fltk::{self, prelude::*};
use std::{thread, error::Error};
use shairport_view_rs::{gui, meta_gen::{self, Metadata}};


fn main() -> Result<(), Box<dyn Error>> {
    let app = fltk::app::App::default();
    let (mut main_win, mut ui) = gui::make_window();
    main_win.show();

    // Closure to define what to do with a piece of Metadata
    // For this binary, we want to update the ui
    let meta_handler = move |metadata: Metadata| -> Result<(), Box<dyn Error>> {
        match metadata {
            Metadata::Track(track) => ui.set_title(&track),
            Metadata::Artist(artist) => ui.set_artist(&artist),
            Metadata::Album(album) => ui.set_album(&album),
            Metadata::Art(art) => ui.set_art(art),
        };
        fltk::app::awake();  // Cause main thread app.wait() to return
        Ok(())
    };

    // This thread:
    //  - Pulls metadata XML from a pipe or stdin
    //  - Parses the XML and creates meta_gen::Metadata items
    //  - Calls the provided meta_handler, which acts on the received Metadata items
    let metadata_thread = thread::spawn(move ||
        meta_gen::parse_metadata(None, meta_handler));

    // App loop needs to redraw whenever it wakes up.
    // The metadata handler should call fltk::app::awake() to cause a redraw, after it updates ui
    // Also check for the metadata thread close (happens when the pipe it reads is closed)
    while app.wait() {
        main_win.redraw();
        if metadata_thread.is_finished() { break }
    }

    Ok(())
}