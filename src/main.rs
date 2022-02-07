mod output;
mod common;
mod audio;

use std::{env, fs, thread};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use console::{Key, Term};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::errors::{Error, Result};
use walkdir::{DirEntry, Error as WDError, WalkDir};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::{Hint, ProbedMetadata};
use dialoguer::{Select, theme::ColorfulTheme};
use symphonia::default::get_codecs;
use common::TrackData;
use crate::output::AudioOutput;

fn main() -> Result<()>{
    let term = Term::stdout();
    term.write_line("Hello, world!")?;
    // scan directory for all files
    // filter by file extension
    // scan each file for metadata
    // print out metadata

    const music_dir: &str = "/Users/joshua.marinacci/Music/Music/Media.localized/Music";

    let mut tracks:Vec<TrackData> = audio::scan_for_tracks(music_dir);

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


    let (send,rec):(Sender<String>, Receiver<String>) = channel();

    let sel = good_tracks[selection];
    let track = sel.clone();
    term.write_line(&format!("going to play {:?}", sel))?;
    let mut playing = true;
    let handler = thread::spawn(move || {
        // println!("in the audio thread");
        let mut audio_output:Option<Box<dyn output::AudioOutput>> = None;
        audio::play_audio(&track, &mut audio_output, rec);
    });


    loop {
        term.write_line(&format!("playing = {}   p=toggle play/pause  q=quit", playing))?;
        if let Ok(key) = term.read_key() {
            match key {
                console::Key::Char('p') => {
                    send.send(String::from("playpause")).unwrap();
                    playing = !playing
                }
                console::Key::Char('q') => {
                    send.send(String::from("quit")).unwrap();
                    handler.join().expect("crashed waiting for the audio thread");
                    break;
                }
                _ => {}
            }
        };
    }
    Ok(())
}
