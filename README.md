# rust_cli_musicplayer

A simple command line music player in Rust, built to learn about Rust audio apis.  Should run on anything.

# usage
Assuming you have a directory somewhere with a bunch of music in it that's also tagged, check out
this repo and do:

```rust
cargo run my_music_dir/somewhere/awesome
```

It will then recursively scan for audio files that it understands (generally MP3, AAC, M4A, WAV, or anything
else understood by the symphonia library).  It will collect this into a list of albums and artists and songs. 
Then choose the album and song and it will play using the TUI.

* Uses [Symphonia](https://docs.rs/symphonia/latest/symphonia/) for audio parsing and decoding.
* Uses [CPAL](https://docs.rs/cpal/latest/cpal/) for cross platform audio playback.
