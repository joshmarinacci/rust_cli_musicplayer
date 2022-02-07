use walkdir::{DirEntry, WalkDir};
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use symphonia::default::get_codecs;
use std::thread;
use std::time::Duration;
use symphonia::core::formats::FormatReader;
use symphonia::core::codecs::Decoder;
use crate::{AudioOutput, CODEC_TYPE_NULL, DecoderOptions, Error, FormatOptions, get_or, Hint, MediaSourceStream, MetadataOptions, output, ProbedMetadata, StandardTagKey, TrackData};

pub enum AudioCommand {
    Play(TrackData),
    TogglePlayPause,
    Quit
}

pub fn scan_for_tracks(music_dir: &str) -> Vec<TrackData> {
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

pub fn play_audio(track: &TrackData, audio_output: &mut Option<Box<dyn AudioOutput>>, rec: Receiver<AudioCommand>) -> crate::Result<()>{
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

        // let tb = track.codec_params.time_base;
        // let dur = track.codec_params.n_frames.map(|frames|track.codec_params.start_ts+frames);

        // Store the track identifier, it will be used to filter packets.
        let track_id = track.id;

        println!("got the track id {}",track_id);
        let mut running = true;
        loop {
            if let Ok(msg) = rec.try_recv() {
                // println!("got the play pause over here");
                match msg {
                    AudioCommand::Play(track) => {
                        println!("got a new track to play {}", get_or(&track.title,"unknown title"));
                    }
                    AudioCommand::TogglePlayPause => running = !running,
                    AudioCommand::Quit => {
                        running = false;
                        break;
                    }
                }
            }
            if !running {
                //sleep for 10th of a second then continue;
                thread::sleep(Duration::from_millis(100));
                continue;
            }
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
                        // println!("trying to open a device");
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
