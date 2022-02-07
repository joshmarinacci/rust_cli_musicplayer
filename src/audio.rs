use walkdir::{DirEntry, WalkDir};
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use symphonia::default::{get_codecs, get_probe};
use std::thread;
use std::time::Duration;
use symphonia::core::formats::FormatReader;
use symphonia::core::codecs::Decoder;
use symphonia::core::meta::Metadata;
use symphonia::core::probe::ProbeResult;
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
    println!("scainng the dir {}",music_dir);
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
            hint.with_extension("m4a");
            let meta_opts: MetadataOptions = Default::default();
            let fmt_opts: FormatOptions = Default::default();
            if let Ok(probed) = &mut get_probe()
                .format(&hint, mss, &fmt_opts, &meta_opts) {
                // println!("success");
                // println!("checking the metadata 1 {:?}",&probed.metadata.get());
                // println!("checking the metadata 2 {:?}",&probed.format.metadata());
                let mut track = TrackData {
                    path: ent.path().to_path_buf(),
                    artist: None,
                    album: None,
                    title: None,
                    number: None,
                    total: None
                };
                process_metadata(&mut track,&mut probed.format.metadata());
                if let Some(md) = &probed.metadata.get() {
                    process_metadata(&mut track, &md);
                }
                tracks.push(track);
            } else {
                println!("failed");
            }

        }
    }

    return tracks;

}

fn process_metadata(track: &mut TrackData, md2: &Metadata)  {
    if let Some(md) = md2.current() {
        for tag in md.tags() {
            // println!("tag {} = {}   or maybe {:?}",tag.key , tag.value, tag.std_key);
            if let Some(std) = tag.std_key {
                // println!("std {:?} = {}",std, tag.value);
                match std {
                    StandardTagKey::Album => track.album = Some(tag.value.to_string()),
                    StandardTagKey::Artist => track.artist = Some(tag.value.to_string()),
                    StandardTagKey::TrackTitle => track.title = Some(tag.value.to_string()),
                    StandardTagKey::TrackNumber => {
                        // println!("have track number {}",tag.value);
                        // if tag.value.to_string().contains('/') {
                            // println!("it has a slash. must split it")
                        // } else {
                            track.number = Some(tag.value.to_string())
                        // }
                    },
                    StandardTagKey::TrackTotal => {
                        println!("have track total {}", tag.value);
                        track.total = Some(tag.value.to_string());
                    }
                    _ => {
                        // println!("other");
                    }
                }
            }
        }
    }
}

struct AudioContext {
    decoder: Box<dyn Decoder>,
    track_id: u32,
    probe_result: ProbeResult,
}
pub fn play_audio(track: &TrackData, audio_output: &mut Option<Box<dyn AudioOutput>>, rec: Receiver<AudioCommand>) -> crate::Result<()>{
    let mut ctx = open_audio_track(track);
    let mut running = true;
    loop {
        if let Ok(msg) = rec.try_recv() {
            // println!("got the play pause over here");
            match msg {
                AudioCommand::Play(track) => {
                    ctx = open_audio_track(&track);
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
        let packet = match ctx.probe_result.format.next_packet() {
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
        if packet.track_id() != ctx.track_id {
            println!("continuing");
            continue;
        }
        match ctx.decoder.decode(&packet) {
            Ok(decoded) => {
                // Consume the decoded audio samples (see below).
                // println!("got some samples {}", decoded.frames());
                if audio_output.is_none() {
                    // println!("trying to open a device");
                    // Get the audio buffer specification. This is a description of the decoded
                    // audio buffer's sample format and sample rate.
                    let spec = *decoded.spec();
                    // println!("spec is {:?}",spec);

                    // Get the capacity of the decoded buffer. Note that this is capacity, not
                    // length! The capacity of the decoded buffer is constant for the life of the
                    // decoder, but the length is not.
                    let duration = decoded.capacity() as u64;
                    // println!("duraction is {}",duration);

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
    Ok(())
}

fn open_audio_track(track: &TrackData) -> AudioContext {
    let src = File::open(&track.path).expect("couldnt open the file");
    let mss = MediaSourceStream::new(Box::new(src), Default::default());


    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    let probe_result = get_probe().format(&hint, mss, &fmt_opts, &meta_opts).unwrap();

        // Find the first audio track with a known (decodeable) codec.
    let track = probe_result.format.tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");
    let track_id = track.id;

        // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();
        // Create a decoder for the track.
    let decoder = get_codecs().make(&track.codec_params, &dec_opts).unwrap();
    AudioContext {
        decoder,
        probe_result,
        track_id:track_id,
    }
}
