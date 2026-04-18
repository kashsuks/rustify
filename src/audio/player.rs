use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::{fs::File, io::BufReader, path::Path};

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    volume: f32,
}

impl Player {
    pub fn new() -> Self {
        let (_stream, handle) = OutputStream::try_default().expect("No audio output device found");
        let sink = Sink::try_new(&handle).unwrap();
        let volume = 1.0;
        sink.set_volume(volume);
        Self {
            _stream,
            handle,
            sink,
            volume,
        }
    }

    pub fn load(&mut self, path: &Path) {
        self.sink.stop();
        self.sink = Sink::try_new(&self.handle).unwrap();
        self.sink.set_volume(self.volume);
        let file = BufReader::new(File::open(path).unwrap());
        let source = Decoder::new(file).unwrap();
        self.sink.append(source);
        self.sink.pause();
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn is_done(&self) -> bool {
        self.sink.empty()
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.sink.set_volume(self.volume);
    }
}
