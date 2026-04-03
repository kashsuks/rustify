use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::{fs::File, io::BufReader, path::Path};

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
}

impl Player {
    pub fn new() -> Self {
        let (_stream, handle) = OutputStream::try_default()
            .expect("No audio output device found");
        let sink = Sink::try_new(&handle).unwrap();
        Self {
            _stream,
            handle,
            sink,
        }
    }

    pub fn load(&mut self, path: &Path) {
        self.sink.stop();
        self.sink = Sink::try_new(&self.handle).unwrap();
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
}
