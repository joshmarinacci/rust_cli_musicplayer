mod output;

use std::{env, fs};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Write, BufReader, BufRead};
use std::path::{Path, PathBuf};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::errors::{Error, Result};
use walkdir::{DirEntry, WalkDir, Error as WDError};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::{Hint, ProbedMetadata};
use dialoguer::{theme::ColorfulTheme, Select};
use symphonia::default::get_codecs;
use crate::output::AudioOutput;

#[derive(Debug)]
struct TrackData {
    path:PathBuf,
    artist:Option<String>,
    album:Option<String>,
    title:Option<String>,
}

fn main() -> Result<()>{
    println!("Hello, world!");
    // scan directory for all files
    // filter by file extension
    // scan each file for metadata
    // print out metadata

    const music_dir: &str = "/Users/joshua.marinacci/Music/Music/Media.localized/Music";

    let mut tracks:Vec<TrackData> = scan_for_tracks(music_dir);

    let good_tracks:Vec<&TrackData> = tracks.iter().filter(|t|t.title != None).collect();


    // let line_str:String = tracks.iter().map(|t|format!("{:?}", t.title.unwrap_or_default())).collect();
    // for track in tracks {
    //     println!("track = {:?}  by {:?}",track.title.unwrap_or_default(), track.artist.unwrap_or_default());
    // }

    // let slc = tracks.as_slice();

    // let good_tracks:Vec<&TrackData> = tracks.iter().filter(|t|t.title != None).collect();
    // let items:Vec<String> = good_tracks.iter().map(|t| t.title.unwrap_or_default()).collect();
    // let items = tracks.iter().map(|t|t.title.unwrap_or_default()).;

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a song")
        .default(0)
        .items(good_tracks.as_slice())
        .interact()?;

    let sel = good_tracks[selection];
    println!("going to play {:?}",sel);

    let mut audio_output:Option<Box<dyn output::AudioOutput>> = None;
    play_audio(sel, &mut audio_output);
    Ok(())
}

fn play_audio(track: &TrackData, audio_output: &mut Option<Box<dyn AudioOutput>>) -> Result<()>{
    let src = File::open(&track.path).expect("couldnt open the file");
    let mss = MediaSourceStream::new(Box::new(src), Default::default());


    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    if let Ok(probed) = &mut symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts) {
        // let mut format = &probed.format;

        // Find the first audio track with a known (decodeable) codec.
        let track = probed.format.tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .expect("no supported audio tracks");

        // Use the default options for the decoder.
        let dec_opts: DecoderOptions = Default::default();

        // Create a decoder for the track.
        let mut decoder = get_codecs().make(&track.codec_params, &dec_opts)?;

        let tb = track.codec_params.time_base;
        let dur = track.codec_params.n_frames.map(|frames|track.codec_params.start_ts+frames);

        // Store the track identifier, it will be used to filter packets.
        let track_id = track.id;

        println!("got the track id {}",track_id);
        loop {
            let packet = match probed.format.next_packet() {
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

            if packet.track_id() != track_id {
                println!("continuing");
                continue;
            }
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Consume the decoded audio samples (see below).
                    // println!("got some samples {}", decoded.frames());
                    if audio_output.is_none() {
                        println!("trying to open a device");
                        // Get the audio buffer specification. This is a description of the decoded
                        // audio buffer's sample format and sample rate.
                        let spec = *decoded.spec();
                        println!("spec is {:?}",spec);

                        // Get the capacity of the decoded buffer. Note that this is capacity, not
                        // length! The capacity of the decoded buffer is constant for the life of the
                        // decoder, but the length is not.
                        let duration = decoded.capacity() as u64;
                        println!("duraction is {}",duration);

                        // Try to open the audio output.
                        audio_output.replace(output::try_open(spec, duration).unwrap());

                    } else {
                        // println!("still open");
                    }
                    if let Some(audio_output) = audio_output {
                        audio_output.write(decoded).unwrap()
                    }
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
    }
    Ok(())

}

impl Display for TrackData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match &self.title {
            None => "unknown",
            Some(data) => data,
        };
        f.write_str(str)
    }
}
// impl ToString for TrackData {
//     fn to_string(&self) -> String {
//         "a string is here".to_string()
//     }
// }

fn scan_for_tracks(music_dir: &str) -> Vec<TrackData> {
    fn is_hidden(entry:&DirEntry) -> bool {
        entry.file_name().to_str().map(|s|s.starts_with(".")).unwrap_or(false)
    }
    let mut tracks:Vec<TrackData> = vec![];

    let walker = WalkDir::new(music_dir).into_iter();
    for entry in walker.filter_entry(|e|!is_hidden(e)) {
        let ent = entry.unwrap();
        let pth = ent.path();
        // println!("{} ", pth.display());
        if pth.is_file() {
            println!("scanning the file {}",pth.file_name().unwrap_or_default().to_str().unwrap_or_default());
            let src = File::open(pth).expect("couldnt open the file");
            let mss = MediaSourceStream::new(Box::new(src), Default::default());


            let mut hint = Hint::new();
            hint.with_extension("mp3");
            let meta_opts: MetadataOptions = Default::default();
            let fmt_opts: FormatOptions = Default::default();
            if let Ok(probed) = &mut symphonia::default::get_probe()
                .format(&hint, mss, &fmt_opts, &meta_opts) {
                tracks.push(process_metadata(&mut probed.metadata, ent.path().to_path_buf()));
            }

        }
    }

    return tracks;

}

fn process_metadata(probed: &mut ProbedMetadata, buf: PathBuf) -> TrackData {
    let mut track = TrackData {
        path:buf,
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
