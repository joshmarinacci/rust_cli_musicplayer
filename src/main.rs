use std::{env, fs};
use std::fs::File;
use std::io::{Write, BufReader, BufRead};
use std::path::Path;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use walkdir::{DirEntry, WalkDir, Error as WDError};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::{Hint, ProbedMetadata};
use symphonia::core::errors::Error;

#[derive(Debug)]
struct TrackData {
    // path:Path,
    artist:Option<String>,
    album:Option<String>,
    title:Option<String>,
}

fn main() -> Result<(),Error>{
    println!("Hello, world!");
    // scan directory for all files
    // filter by file extension
    // scan each file for metadata
    // print out metadata

    const music_dir: &str = "/Users/joshua.marinacci/Music/Music/Media.localized/Music";
    fn is_hidden(entry:&DirEntry) -> bool {
        entry.file_name().to_str().map(|s|s.starts_with(".")).unwrap_or(false)
    }

    let mut tracks:Vec<TrackData> = vec![];

    let walker = WalkDir::new(music_dir).into_iter();
    for entry in walker.filter_entry(|e|!is_hidden(e)) {
        let ent = entry.unwrap();
        let pth = ent.path();
        println!("{} ", pth.display());
        if pth.is_file() {
            println!("scanning the file {}",pth.display());
            let src = File::open(pth).expect("couldnt open the file");
            let mss = MediaSourceStream::new(Box::new(src), Default::default());


            let mut hint = Hint::new();
            hint.with_extension("mp3");
            let meta_opts: MetadataOptions = Default::default();
            let fmt_opts: FormatOptions = Default::default();
            if let Ok(probed) = &mut symphonia::default::get_probe()
                .format(&hint, mss, &fmt_opts, &meta_opts) {
                tracks.push(process_metadata(&mut probed.metadata));
            }
            /*
            let mut format = probed.format;

            // Find the first audio track with a known (decodeable) codec.
            let track = format.tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                .expect("no supported audio tracks");

            // Use the default options for the decoder.
            let dec_opts: DecoderOptions = Default::default();

            // Create a decoder for the track.
            let mut decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &dec_opts)
                .expect("unsupported codec");

            // Store the track identifier, it will be used to filter packets.
            let track_id = track.id;

            println!("got the track id {}",track_id);
            let mut count = 0;
            loop {
                let packet = match format.next_packet() {
                    Ok(packet) => packet,
                    Err(Error::ResetRequired) => {
                        println!("reset required");
                        unimplemented!()
                    }
                    Err(err) => {
                        println!("error . end of stream?");
                        break;
                    }
                };
                // println!("got packet {}",count);
                count += 1;

                while !format.metadata().is_latest() {
                    println!("got metadata");
                    format.metadata().pop();
                    if let Some(rev) = format.metadata().current() {
                        println!("rev is {:?}",rev);
                        // Consume the new metadata at the head of the metadata queue.
                    }
                }
                if packet.track_id() != track_id {
                    println!("continuing");
                    continue;
                }
                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        // Consume the decoded audio samples (see below).
                        // println!("got some samples {}", decoded.frames());
                    }
                    Err(Error::IoError(_)) => {
                        println!("io error");
                        // The packet failed to decode due to an IO error, skip the packet.
                        continue;
                    }
                    Err(Error::DecodeError(_)) => {
                        println!("decode error");
                        // The packet failed to decode due to invalid data, skip the packet.
                        continue;
                    }
                    Err(err) => {
                        // An unrecoverable error occured, halt decoding.
                        println!("{}", err);
                    }
                }

            }
            */
        }
    }


    // let line_str:String = tracks.iter().map(|t|format!("{:?}", t.title.unwrap_or_default())).collect();
    for track in tracks {
        println!("track = {:?}  by {:?}",track.title.unwrap_or_default(), track.artist.unwrap_or_default());
    }
    // println!("final list of tracks {:?}", line_str);

    Ok(())
}

fn process_metadata(probed: &mut ProbedMetadata) -> TrackData {
    let mut track = TrackData {
        artist: None,
        album: None,
        title: None
    };
    if let Some(md) = probed.get() {
        // println!("proped returned {:?}", md);
        if let Some(md) = md.current() {
            for tag in md.tags() {
                // println!("tag {} = {}   or maybe {:?}",tag.key , tag.value, tag.std_key);
                if let Some(std) = tag.std_key {
                    // println!("std {:?} = {}",std, tag.value);
                    match std {
                        StandardTagKey::Album => track.album = Some(tag.value.to_string()),
                        StandardTagKey::Artist => track.artist = Some(tag.value.to_string()),
                        StandardTagKey::TrackTitle => track.title = Some(tag.value.to_string()),
                        _ => {
                            // println!("other");
                        }
                    }
                }
            }
        }
    }

    return track;
}
