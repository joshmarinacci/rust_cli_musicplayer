mod output;
mod common;
mod audio;

use std::{env, fs, thread};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use console::{Key, Term};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::{FormatOptions, Track};
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

pub struct MusicLibrary {
    pub tracks:Vec<Rc<TrackData>>,
    pub artists:HashMap<String,Vec<Rc<TrackData>>>,
    pub albums:HashMap<String,Vec<Rc<TrackData>>>,
}

impl MusicLibrary {
    fn from_tracks<'a>(tracks: Vec<TrackData>) -> MusicLibrary {
        let mut lib = MusicLibrary {
            tracks: vec![],
            artists: Default::default(),
            albums: Default::default(),
        };

        for track in &tracks {
            let tr = Rc::new(track.clone());
            lib.tracks.push(Rc::clone(&tr));
            if let Some(st) = &track.artist {
                if !lib.artists.contains_key(st) {
                    lib.artists.insert(st.clone(),vec![]);
                }
                let mut artist = lib.artists.get_mut(st).unwrap();
                artist.push(Rc::clone(&tr));
            }
            if let Some(st) = &track.album {
                if !lib.albums.contains_key(st) {
                    lib.albums.insert(st.clone(),vec![]);
                }
                let mut album = lib.albums.get_mut(st).unwrap();
                album.push(Rc::clone(&tr));
            }
        }

        lib
    }
}

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


    let mut tracks:Vec<TrackData> = audio::scan_for_tracks(music_dir);
    let lib = MusicLibrary::from_tracks(tracks);
    // let good_tracks:Vec<&TrackData> = lib.tracks.iter().filter(|t|t.title != None).collect();
    start_interface(lib)
}
fn start_interface(lib: MusicLibrary) -> Result<()>{
    let term = Term::stdout();
    let track = choose_track(&lib)?;
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
                    current_track = choose_track(&lib)?;
                    send.send(AudioCommand::Play(current_track.clone()));
                }
                _ => {}
            }
        };
    }

    return Ok(())
}

fn choose_track(lib: &MusicLibrary) -> Result<TrackData> {
    let albums = lib.albums.keys().collect::<Vec<&String>>();
    let album_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an album")
        .default(0)
        .items(albums.as_slice())
        .interact()?;

    let album_name = albums[album_index];

    println!("chose album name {}", album_name);

    let traks = lib.albums.get(album_name).unwrap();
    let track_index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("choose track")
        .default(0)
        .items(traks)
        .interact()?;

    let track = (*traks[track_index].clone()).clone();
    Ok(track)
}
