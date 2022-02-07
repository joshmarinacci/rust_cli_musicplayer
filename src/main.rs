mod output;
mod common;
mod audio;

use std::{env, fs, thread};
use std::borrow::Borrow;
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
use crate::audio::AudioCommand;
use crate::common::get_or;
use crate::output::AudioOutput;

fn main() -> Result<()>{
    let args:Vec<String> = env::args().collect();
    for arg in args.iter() {
        println!("arg {}",arg);
    }
    if let None = args.get(1) {
        println!("you must specify a directory full of music");
        return Ok(());
    }
    let music_dir = args.get(1).unwrap();
    let term = Term::stdout();


    let mut tracks:Vec<TrackData> = audio::scan_for_tracks(music_dir);
    let good_tracks:Vec<&TrackData> = tracks.iter().filter(|t|t.title != None).collect();
    let track = choose_track(&good_tracks)?;
    let (send,rec):(Sender<AudioCommand>, Receiver<AudioCommand>) = channel();

    let mut playing = true;
    let mut current_track = track.clone();
    let handler = thread::spawn(move || {
        let mut audio_output:Option<Box<dyn output::AudioOutput>> = None;
        audio::play_audio(&track, &mut audio_output, rec);
    });


    loop {
        term.clear_screen()?;
        term.write_line(&format!("{}  /  {}  -- {}",
                                 get_or(&current_track.title,"???"),
                                 get_or(&current_track.artist, "???"),
            get_or(&current_track.number, "???"),

        ))?;
        term.write_line(&format!("playing = {}   p=toggle play/pause  q=quit", playing))?;
        if let Ok(key) = term.read_key() {
            match key {
                console::Key::Char('p') => {
                    send.send(AudioCommand::TogglePlayPause).unwrap();
                    playing = !playing
                }
                console::Key::Char('q') => {
                    send.send(AudioCommand::Quit).unwrap();
                    handler.join().expect("crashed waiting for the audio thread");
                    break;
                }
                console::Key::Char('c') => {
                    current_track = choose_track(&good_tracks)?;
                    send.send(AudioCommand::Play(current_track.clone()));
                }
                _ => {}
            }
        };
    }
    Ok(())
}

fn choose_track(good_tracks: &Vec<&TrackData>) -> Result<TrackData> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a song")
        .default(0)
        .items(good_tracks.as_slice())
        .interact()?;

    let sel = good_tracks[selection].clone();
    Ok(sel)
}
