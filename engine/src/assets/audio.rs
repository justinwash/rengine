use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::assets::AssetError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioBus {
    Music,
    Effects,
    Ui,
    Ambient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioId(pub(crate) usize);

#[derive(Debug, Clone)]
pub struct AudioClip {
    pub id: AudioId,
    pub path: PathBuf,
}

struct AudioData {
    bytes: Arc<[u8]>,
}

struct ActiveSink {
    bus: AudioBus,
    sink: Sink,
    base_volume: f32,
}

pub(crate) struct AudioSystem {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    clips: Vec<AudioData>,
    cache: HashMap<PathBuf, AudioClip>,
    timestamps: HashMap<PathBuf, SystemTime>,
    master_volume: Cell<f32>,
    bus_volumes: RefCell<HashMap<AudioBus, f32>>,
    active_sinks: RefCell<Vec<ActiveSink>>,
    music_sink: RefCell<Option<ActiveSink>>,
    silent: bool,
    allow_headless_noop: bool,
}

impl AudioSystem {
    pub fn new(headless: bool) -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                _stream: Some(stream),
                handle: Some(handle),
                clips: Vec::new(),
                cache: HashMap::new(),
                timestamps: HashMap::new(),
                master_volume: Cell::new(if headless { 0.0 } else { 1.0 }),
                bus_volumes: RefCell::new(default_bus_volumes()),
                active_sinks: RefCell::new(Vec::new()),
                music_sink: RefCell::new(None),
                silent: headless,
                allow_headless_noop: headless,
            },
            Err(error) => {
                log::warn!("Audio output unavailable: {error}");
                Self {
                    _stream: None,
                    handle: None,
                    clips: Vec::new(),
                    cache: HashMap::new(),
                    timestamps: HashMap::new(),
                    master_volume: Cell::new(if headless { 0.0 } else { 1.0 }),
                    bus_volumes: RefCell::new(default_bus_volumes()),
                    active_sinks: RefCell::new(Vec::new()),
                    music_sink: RefCell::new(None),
                    silent: headless,
                    allow_headless_noop: headless,
                }
            }
        }
    }

    pub fn register_clip(&mut self, path: PathBuf, bytes: Arc<[u8]>) -> AudioClip {
        if let Some(existing) = self.cache.get(&path) {
            return existing.clone();
        }

        let clip = AudioClip {
            id: AudioId(self.clips.len()),
            path: path.clone(),
        };
        self.clips.push(AudioData { bytes });
        self.cache.insert(path.clone(), clip.clone());
        if let Ok(modified) = file_modified_time(&path) {
            self.timestamps.insert(path, modified);
        }
        clip
    }

    pub fn play(&self, clip: &AudioClip) -> Result<(), AssetError> {
        self.play_on_bus(AudioBus::Effects, clip, 1.0)
    }

    pub fn play_on_bus(
        &self,
        bus: AudioBus,
        clip: &AudioClip,
        volume: f32,
    ) -> Result<(), AssetError> {
        self.cleanup_sinks();
        if self.handle.is_none() && self.allow_headless_noop {
            self.decode_clip(clip)?;
            return Ok(());
        }
        let sink = self.new_sink(&clip.path)?;
        sink.set_volume(self.final_volume(bus, volume));
        let source = self.decode_clip(clip)?;
        sink.append(source);
        self.active_sinks.borrow_mut().push(ActiveSink {
            bus,
            sink,
            base_volume: volume.max(0.0),
        });
        Ok(())
    }

    pub fn play_music(&self, clip: &AudioClip) -> Result<(), AssetError> {
        self.play_music_with_volume(clip, 1.0)
    }

    pub fn play_music_with_volume(&self, clip: &AudioClip, volume: f32) -> Result<(), AssetError> {
        self.stop_music();

        if self.handle.is_none() && self.allow_headless_noop {
            self.decode_clip(clip)?;
            return Ok(());
        }

        let sink = self.new_sink(&clip.path)?;
        sink.set_volume(self.final_volume(AudioBus::Music, volume));
        let source = self.decode_clip(clip)?;
        sink.append(source.repeat_infinite());
        sink.play();
        *self.music_sink.borrow_mut() = Some(ActiveSink {
            bus: AudioBus::Music,
            sink,
            base_volume: volume.max(0.0),
        });
        Ok(())
    }

    pub fn stop_music(&self) {
        if let Some(active) = self.music_sink.borrow_mut().take() {
            active.sink.stop();
        }
    }

    pub fn pause_music(&self) {
        if let Some(active) = self.music_sink.borrow().as_ref() {
            active.sink.pause();
        }
    }

    pub fn resume_music(&self) {
        if let Some(active) = self.music_sink.borrow().as_ref() {
            active.sink.play();
        }
    }

    pub fn stop_bus(&self, bus: AudioBus) {
        if bus == AudioBus::Music {
            self.stop_music();
            return;
        }

        let mut sinks = self.active_sinks.borrow_mut();
        for active in sinks.iter() {
            if active.bus == bus {
                active.sink.stop();
            }
        }
        sinks.retain(|active| active.bus != bus && !active.sink.empty());
    }

    pub fn set_master_volume(&self, volume: f32) {
        self.master_volume.set(if self.silent { 0.0 } else { volume.max(0.0) });
        self.refresh_sink_volumes();
    }

    pub fn master_volume(&self) -> f32 {
        self.master_volume.get()
    }

    pub fn set_bus_volume(&self, bus: AudioBus, volume: f32) {
        self.bus_volumes.borrow_mut().insert(bus, volume.max(0.0));
        self.refresh_sink_volumes();
    }

    pub fn bus_volume(&self, bus: AudioBus) -> f32 {
        *self.bus_volumes.borrow().get(&bus).unwrap_or(&1.0)
    }

    pub fn reload_changed(&mut self) -> Vec<Result<PathBuf, AssetError>> {
        let watched: Vec<(PathBuf, AudioClip)> = self
            .cache
            .iter()
            .map(|(path, clip)| (path.clone(), clip.clone()))
            .collect();
        let mut results = Vec::new();

        for (path, clip) in watched {
            let Ok(modified) = file_modified_time(&path) else {
                continue;
            };
            let changed = self
                .timestamps
                .get(&path)
                .map(|known| modified > *known)
                .unwrap_or(true);
            if !changed {
                continue;
            }

            match fs::read(&path) {
                Ok(bytes) => {
                    if let Some(slot) = self.clips.get_mut(clip.id.0) {
                        slot.bytes = Arc::from(bytes.into_boxed_slice());
                        self.timestamps.insert(path.clone(), modified);
                        results.push(Ok(path));
                    } else {
                        results.push(Err(AssetError::audio_message(
                            &clip.path,
                            "invalid audio clip handle during reload",
                        )));
                    }
                }
                Err(source) => results.push(Err(AssetError::Io { path, source })),
            }
        }

        results
    }

    fn cleanup_sinks(&self) {
        self.active_sinks
            .borrow_mut()
            .retain(|active| !active.sink.empty());
    }

    fn refresh_sink_volumes(&self) {
        self.cleanup_sinks();
        let master = self.master_volume();
        let bus_volumes = self.bus_volumes.borrow();

        if let Some(active) = self.music_sink.borrow().as_ref() {
            let bus = *bus_volumes.get(&AudioBus::Music).unwrap_or(&1.0);
            active.sink.set_volume(master * bus * active.base_volume);
        }

        for active in self.active_sinks.borrow().iter() {
            let bus = *bus_volumes.get(&active.bus).unwrap_or(&1.0);
            active.sink.set_volume(master * bus * active.base_volume);
        }
    }

    fn final_volume(&self, bus: AudioBus, volume: f32) -> f32 {
        if self.silent {
            return 0.0;
        }
        self.master_volume() * self.bus_volume(bus) * volume.max(0.0)
    }

    fn new_sink(&self, path: &Path) -> Result<Sink, AssetError> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| AssetError::audio_message(path, "no audio output device available"))?;
        Sink::try_new(handle).map_err(|error| AssetError::audio_message(path, error.to_string()))
    }

    fn decode_clip(&self, clip: &AudioClip) -> Result<Decoder<Cursor<Vec<u8>>>, AssetError> {
        let bytes = self
            .clips
            .get(clip.id.0)
            .ok_or_else(|| AssetError::audio_message(&clip.path, "invalid audio clip handle"))?
            .bytes
            .to_vec();
        Decoder::new(Cursor::new(bytes))
            .map_err(|error| AssetError::audio_message(&clip.path, error.to_string()))
    }
}

fn default_bus_volumes() -> HashMap<AudioBus, f32> {
    HashMap::from([
        (AudioBus::Music, 1.0),
        (AudioBus::Effects, 1.0),
        (AudioBus::Ui, 1.0),
        (AudioBus::Ambient, 1.0),
    ])
}

fn file_modified_time(path: &Path) -> Result<SystemTime, std::io::Error> {
    fs::metadata(path)?.modified()
}