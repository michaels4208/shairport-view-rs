use amxml::sax::*;
use std::{fmt::{self, Display}, env, fs::File, io::{self, BufRead, BufReader, Write}, error::Error};
use base64::{Engine as _, engine::general_purpose};
use fltk::{prelude::{ImageExt, FltkError}, image::{SharedImage, JpegImage, PngImage}};

/// Errors coming from Metadata operations
#[derive(Debug, Clone)]
pub struct MetadataError(String);

impl Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error from the metadata thread: {}", self.0)
    }
}

impl Error for MetadataError {}

/// Every recognized piece of metadata coming from the source gets represented with this enum
/// Support errors, so that they can be sent over the metadata channel
pub enum Metadata {
    Track(String),
    Artist(String),
    Album(String),
    Art(SharedImage),
}

impl Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Track(track) => write!(f, "Track: {}", track),
            Self::Artist(artist) => write!(f, "Artist: {}", artist),
            Self::Album(album) => write!(f, "Album: {}", album),
            Self::Art(art) => {
                let (w, h) = (art.w(), art.h());
                write!(f, "Art is a {w}x{h} image")
            },
        }
    }
}

// Choosing not to implement this for Metadata,
// because it does not need to be part of the public interface
fn make_art(art: Vec<u8>) -> Result<Metadata, Box<dyn Error>> {
    if art.len() == 0 {
        Err(Box::new(MetadataError("Invalid art bytes".to_owned())))
    }
    else if art[0] == 0xff {
        Ok(Metadata::Art(SharedImage::from_image(JpegImage::from_data(&art)?)?))
    }
    else {
        Ok(Metadata::Art(SharedImage::from_image(PngImage::from_data(&art)?)?))
    }
}

/// Wrapper for metadata parsing code.
/// Handles returned errors by printing to stdout, then exiting
pub fn parse_metadata(pipe_path: Option<&str>, meta_handler: impl FnMut(Metadata) -> Result<(), Box<dyn Error>>) {
    match parse_metadata_ret_err(pipe_path, meta_handler) {
        Ok(_) => println!("Metadata handling thread exiting with success"),
        Err(err) => println!("{}", err),
    }
}

/// Function for reading and parsing metadata, then applying a handler func to the resulting data
/// Can be executed as a separate thread
pub fn parse_metadata_ret_err(pipe_path: Option<&str>,
                          mut meta_handler: impl FnMut(Metadata) -> Result<(), Box<dyn Error>>)
    -> Result<(), Box<dyn Error>> {
    // If the pipe_path was provided, use it.
    // If not, there still might be one given as a CLI arg.  Use that.
    let cli_args: Vec<String> = env::args().collect();
    let mut meta_buffer: Box<dyn Iterator<Item = io::Result<String>>> = if pipe_path.is_some() || cli_args.len() > 1 {
        // We're reading from a named pipe at some path
        let pipe_path = match pipe_path {
            Some(pipe_path) => pipe_path,
            None => &cli_args[1],
        };
        Box::new(BufReader::new(File::open(pipe_path)?).lines())
    }
    else {
        Box::new(io::stdin().lock().lines())
    };

    let mut xml_metadata = XMLMetadata::new()?;

    loop {
        match meta_buffer.next() {
            // We got a line of XML metadata from the source.  Parse it
            Some(Ok(xml_line)) if xml_line.len() > 0 => {
                for metadata in xml_metadata.parse_line(&xml_line) {
                    meta_handler(metadata)?
                }
            },

            // No data from the source.  Sleep for a bit.
            Some(Ok(_)) => std::thread::sleep(std::time::Duration::from_millis(50)),

            // Error reading from the source. Stop looping over it.
            Some(Err(_)) => break,
            None => break,
        }
    };
    Ok(())
}

// Struct keeps track of previously parsed XML.
// That way it can be parsed line by line from a pipe
struct XMLMetadata {
    tag_stack: Vec<String>,
    curr_type: String,
    curr_code: String,
    // If an image def is found but cannot be decoded, send this one in its place
    default_art: SharedImage,
}

impl XMLMetadata {
    fn new() -> Result<Self, FltkError> {
        Ok(Self{
            tag_stack: Vec::new(),
            curr_type: "".to_owned(),
            curr_code: "".to_owned(),
            default_art: SharedImage::from_image(PngImage::from_data(include_bytes!("resources/img/no-art.png"))?)?,
        })
    }

    // Parse a line of XML (metadata arg), and return a Vector of Metadata items encountered
    // Mutate self while running, so that tags/type/code state can be kept up to date
    fn parse_line(&mut self, metadata: &str) -> Vec<Metadata> {
        let mut dec = SaxDecoder::new(metadata).unwrap();
        let def_tag: &str = "No tag";
        let mut found_metadata: Vec<Metadata> = Vec::new();

        loop {
            match dec.raw_token() {
                Ok(XmlToken::EOF) => {
                    //println!("End");
                    return found_metadata
                },
                Ok(XmlToken::StartElement{name, ..}) => {
                    //println!("found start={name}");
                    self.tag_stack.push(name);
                },
                Ok(XmlToken::EndElement{name}) => {
                    // println!("found end={name}");

                    if name == "item" {
                        self.curr_type = "".to_string();
                        self.curr_code = "".to_string();
                    }
                },
                Ok(XmlToken::CharData{chardata}) => {
                    // curr_tag is the last start element name we saw
                    let curr_tag: &str = self.tag_stack
                        // Get last element in tag stack, or 0th if none are in it
                        .get(std::cmp::max(self.tag_stack.len(), 1) - 1)
                        .as_ref()
                        .map_or(def_tag, |s| &s[..]);

                    match curr_tag {
                        "type" => self.curr_type = decode_xml_hex(chardata),
                        "code" => self.curr_code = decode_xml_hex(chardata),
                        "data" => {
                            // println!("Found data with ({self.curr_type}, {self.curr_code})");
                            match (self.curr_type.as_str(), self.curr_code.as_str()) {
                                ("core", "asar") => found_metadata.push(Metadata::Artist(decode_xml_b64(chardata))),
                                ("core", "asal") => found_metadata.push(Metadata::Album(decode_xml_b64(chardata))),
                                ("core", "minm") => found_metadata.push(Metadata::Track(decode_xml_b64(chardata))),
                                ("ssnc", "PICT") => {
                                    match make_art(general_purpose::STANDARD.decode(chardata).unwrap()) {
                                        Ok(art) => found_metadata.push(art),
                                        Err(e) => {
                                            _ = io::stdout().write_all(format!("Error when processing metadata: {}", e).as_bytes());
                                            found_metadata.push(Metadata::Art(self.default_art.clone()))
                                        },
                                    }
                                },
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                },
                Ok(XmlToken::ProcInst{..}) => {
                    //println!("found ProcInst=({target}, {inst})");
                },
                Ok(XmlToken::Comment{..}) => {
                    //println!("found comment={comment}");
                },
                _ => {},
            }
        }
    }
}

fn decode_xml_hex(chardata: String) -> String {
    let ret = match hex::decode(chardata.clone()).map(String::from_utf8) {
        Ok(Ok(strdata)) => strdata,
        _ => {
            println!("Got malformed data: {chardata}");
            "XML Data Error".to_owned()
        },
    };
    // println!("Decoded: {ret}");
    ret
}

fn decode_xml_b64(chardata: String) -> String {
    let chardata = chardata.trim();
    // println!("Trying to decode b64: '{chardata}'");
    match general_purpose::STANDARD.decode(chardata) {
        Ok(decoded_bytes) => match String::from_utf8(decoded_bytes) {
            Ok(decoded_string) => decoded_string,
            Err(error) => format!("Error when decoding bytes from XML b64: {error:?}"),
        },
        Err(error) => { format!("Error when decoding XML b64: {error:?}") },
    }
}
