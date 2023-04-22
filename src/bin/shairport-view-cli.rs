use amxml::sax::*;
use std::{fmt::{self, Display}, env, fs::File, io::{self, BufRead, BufReader, Write}, error::Error};
use std::sync::mpsc;
use base64::{Engine as _, engine::general_purpose};
use fltk::{prelude::{ImageExt, FltkError}, image::{SharedImage, JpegImage, PngImage}};

#[derive(Debug, Clone)]
struct MetadataError(String);

impl Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Metadata Error: {}", self.0)
    }
}

impl Error for MetadataError {}

enum Metadata {
    Track(String),
    Artist(String),
    Album(String),
    Art(SharedImage),
}

impl fmt::Display for Metadata {
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

impl Metadata {
    fn make_art(art: Vec<u8>) -> Result<Metadata, Box<dyn std::error::Error>> {
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
}

// Executable for viewing shairport-sync metadata on the command line
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipe_rx = spawn_metadata_channel()?;
    let mut xml_metadata = XMLMetadata::new()?;

    loop {
        // println!("Reading pipe");
        match pipe_rx.try_recv() {
            Ok(xml_line) => {
                // println!("Main thread got XML Data:\n  {xml_line}");
                for metadata in xml_metadata.parse_line(&xml_line) {
                    println!("{}", metadata);
                }
            },
            Err(mpsc::TryRecvError::Empty) => std::thread::sleep(std::time::Duration::from_millis(100)), // println!("No XML Data"),
            Err(mpsc::TryRecvError::Disconnected) => break, // { println!("XML reader died"); break },
        }
    }

    Ok(())
}

/// Struct keeps track of previously parsed XML.
/// That way it can be parsed line by line from a pipe
struct XMLMetadata {
    tag_stack: Vec<String>,
    curr_type: String,
    curr_code: String,
    default_art: SharedImage,
}

impl XMLMetadata {
    fn new() -> Result<Self, FltkError> {
        Ok(Self{
            tag_stack: Vec::new(),
            curr_type: "".to_owned(),
            curr_code: "".to_owned(),
            default_art: SharedImage::from_image(PngImage::from_data(include_bytes!("../resources/img/no-art.png"))?)?,
        })
    }

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
                                    match Metadata::make_art(general_purpose::STANDARD.decode(chardata).unwrap()) {
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

fn spawn_metadata_channel() -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>>
{
    // If a path arg was provided, open the pipe at that path
    let cli_args: Vec<String> = env::args().collect();
    if cli_args.len() > 1 {
        let pipe_handle = BufReader::new(File::open(&cli_args[1])?);
        Ok(spawn_metadata_channel_pipe(pipe_handle))
    }
    else {
        Ok(spawn_metadata_channel_stdin())
    }
}

// Spawn a thread that reads from stdin and sends back data
// Reading from stdin line by line is a blocking operation,
// but reading from the channel this thread returns doesn't have to be
fn spawn_metadata_channel_stdin() -> mpsc::Receiver<String>
{
    let (tx, rx) = mpsc::channel::<String>();
    // Thread that sends non-empty lines from the pipe over a channel
    // Thread sleeps on empty lines, ends on pipe read errors or pipe writer close / end of pipe
    std::thread::spawn(move || {
        let mut metadata_source = io::stdin().lock().lines();

        loop {
            match metadata_source.next() {
                Some(Ok(line)) if line.len() > 0 => tx.send(line).unwrap(),
                Some(Ok(_)) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Some(Err(_)) => break,
                None => break,
            }
        }});
    rx
}

// Spawn a thread that reads from a pipe and sends back data
// Reading from a pipe is a blocking operation,
// but reading from the channel this thread returns doesn't have to be
fn spawn_metadata_channel_pipe(metadata_pipe: BufReader<File>) -> mpsc::Receiver<String>
{
    let (tx, rx) = mpsc::channel::<String>();
    // Thread that sends non-empty lines from the pipe over a channel
    // Thread sleeps on empty lines, ends on pipe read errors or pipe writer close / end of pipe
    std::thread::spawn(move || {
        let mut metadata_source = metadata_pipe.lines();

        loop {
            match metadata_source.next() {
                Some(Ok(line)) if line.len() > 0 => tx.send(line).unwrap(),
                Some(Ok(_)) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Some(Err(_)) => break,
                None => break,
            }
        }});
    rx
}