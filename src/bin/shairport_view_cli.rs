use shairport_view_rs::meta_gen;

// Executable for viewing shairport-sync metadata on the command line
fn main() -> Result<(), Box<dyn std::error::Error>> {
    meta_gen::parse_metadata_ret_err(None, |m| { println!("{}", m); Ok(()) } )
}