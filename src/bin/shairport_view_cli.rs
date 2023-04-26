use shairport_view_rs::meta_gen;

// Executable for viewing shairport-sync metadata on the command line
// Similar to code in shairport_view_gui, but we don't need a separate thread,
// and we just print the metadata items we get
fn main() -> Result<(), Box<dyn std::error::Error>> {
    meta_gen::parse_metadata_ret_err(None, |m| { println!("{}", m); Ok(()) } )
}