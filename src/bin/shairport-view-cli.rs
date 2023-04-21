use amxml::sax::*;
use std::{fmt, env, fs, io::{BufRead, BufReader}};
use std::sync::mpsc;
use base64::{Engine as _, engine::general_purpose};
use fltk::image;

enum Metadata {
    Track(String),
    Artist(String),
    Album(String),
    // Art(image::PngImage),
    Art(Vec<u8>),
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Track(track) => write!(f, "Track: {}", track),
            Self::Artist(artist) => write!(f, "Artist: {}", artist),
            Self::Album(album) => write!(f, "Album: {}", album),
            Self::Art(art) => {
                let l = art.len();
                fs::write(format!("{l}.png"), art).expect("Couldn't write image");
                write!(f, "Wrote image with length {l} to file")
            }
        }
    }
}

fn get_metadata_source<T>() -> Result<T, Box<dyn std::error::Error>>
    where T: BufRead,
{
    // If a path arg was provided, open the pipe at that path
    let cli_args: Vec<String> = env::args().collect();
    if cli_args.len() > 1 {
        let pipe_handle = std::fs::File::open(cli_args[1])?;
        Ok(BufReader::new(pipe_handle).lines())
    }
    else {
        Ok(std::io::stdin().lock())
    }
}

// Executable for viewing shairport-sync metadata on the command line
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata_source = get_metadata_source().unwrap(); // Panic if we can't get a data source
    let pipe_rx = spawn_pipe_channel(metadata_source)?;
    let mut xml_metadata = XMLMetadata::new();

    loop {
        // println!("Reading pipe");
        match pipe_rx.try_recv() {
            Ok(xml_line) => {
                // println!("Main thread got XML Data:\n  {xml_line}");
                let (new_metadata, pipe_closed) = xml_metadata.parse_line(&xml_line);
                for metadata in new_metadata {
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
    curr_code: String
}

impl XMLMetadata {
    fn new() -> Self {
        Self{
            tag_stack: Vec::new(),
            curr_type: "".to_owned(),
            curr_code: "".to_owned(),
        }
    }

    fn parse_line(&mut self, metadata: &str) -> (Vec<Metadata>, bool) {
        let mut dec = SaxDecoder::new(metadata).unwrap();
        let def_tag: &str = "No tag";
        let mut found_metadata: Vec<Metadata> = Vec::new();

        loop {
            match dec.raw_token() {
                Ok(XmlToken::EOF) => {
                    //println!("End");
                    return (found_metadata, true)
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
                                // ("ssnc", "PICT") => found_metadata.push(Metadata::Art(image::PngImage::from_data(decode_xml_b64(chardata).unwrap()))),
                                ("ssnc", "PICT") => found_metadata.push(Metadata::Art(general_purpose::STANDARD.decode(chardata).unwrap())),
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

        (found_metadata, false)
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

// Spawn a thread that reads from a pipe and sends back data
// Reading from a pipe is a blocking operation,
// but reading from the channel this thread returns doesn't have to be
fn spawn_pipe_channel<T>(metadata_source: T) -> Result<mpsc::Receiver<String>, Box<dyn std::error::Error>>
    where T: BufRead,
{
    //let mut pipe_reader = BufReader::new(metadata_pipe).lines();
    let (tx, rx) = mpsc::channel::<String>();
    // Thread that sends non-empty lines from the pipe over a channel
    // Thread sleeps on empty lines, ends on pipe read errors or pipe writer close / end of pipe
    std::thread::spawn(move ||
        loop {
            match metadata_source.next() {
                Some(Ok(line)) if line.len() > 0 => tx.send(line).unwrap(),
                Some(Ok(_)) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Some(Err(_)) => break,
                None => break,
            }
        });
    Ok(rx)
}
