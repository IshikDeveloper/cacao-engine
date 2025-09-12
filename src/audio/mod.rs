// src/audio/mod.rs
use std::collections::HashMap;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use crate::{errors::CacaoError, assets::AudioClip};

pub struct AudioSystem {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sound_sinks: HashMap<String, Sink>,
    music_sink: Option<Sink>,
    master_volume: f32,
    sound_volume: f32,
    music_volume: f32,
}

impl AudioSystem {
    pub fn new() -> Result<Self, CacaoError> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| CacaoError::AudioError(format!("Failed to create audio output stream: {}", e)))?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            sound_sinks: HashMap::new(),
            music_sink: None,
            master_volume: 1.0,
            sound_volume: 1.0,
            music_volume: 1.0,
        })
    }

    pub fn play_sound(&mut self, audio_clip: &AudioClip, loop_sound: bool) -> Result<String, CacaoError> {
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| CacaoError::AudioError(format!("Failed to create audio sink: {}", e)))?;

        let cursor = std::io::Cursor::new(audio_clip.data.clone());
        let source = Decoder::new(cursor)
            .map_err(|e| CacaoError::AudioError(format!("Failed to decode audio: {}", e)))?;

        if loop_sound {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }

        sink.set_volume(self.master_volume * self.sound_volume);
        sink.play();

        // Generate a unique ID for this sound instance
        let sound_id = uuid::Uuid::new_v4().to_string();
        self.sound_sinks.insert(sound_id.clone(), sink);

        Ok(sound_id)
    }

    pub fn play_music(&mut self, audio_clip: &AudioClip, loop_music: bool) -> Result<(), CacaoError> {
        // Stop current music if playing
        if let Some(ref music_sink) = self.music_sink {
            music_sink.stop();
        }

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| CacaoError::AudioError(format!("Failed to create music sink: {}", e)))?;

        let cursor = std::io::Cursor::new(audio_clip.data.clone());
        let source = Decoder::new(cursor)
            .map_err(|e| CacaoError::AudioError(format!("Failed to decode music: {}", e)))?;

        if loop_music {
            sink.append(source.repeat_infinite());
        } else {
            sink.append(source);
        }

        sink.set_volume(self.master_volume * self.music_volume);
        sink.play();

        self.music_sink = Some(sink);
        Ok(())
    }

    pub fn stop_sound(&mut self, sound_id: &str) {
        if let Some(sink) = self.sound_sinks.remove(sound_id) {
            sink.stop();
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(ref music_sink) = self.music_sink {
            music_sink.stop();
        }
        self.music_sink = None;
    }

    pub fn stop_all_sounds(&mut self) {
        for (_, sink) in self.sound_sinks.drain() {
            sink.stop();
        }
    }

    pub fn stop_all(&mut self) {
        self.stop_all_sounds();
        self.stop_music();
    }

    pub fn pause_sound(&mut self, sound_id: &str) {
        if let Some(sink) = self.sound_sinks.get(sound_id) {
            sink.pause();
        }
    }

    pub fn resume_sound(&mut self, sound_id: &str) {
        if let Some(sink) = self.sound_sinks.get(sound_id) {
            sink.play();
        }
    }

    pub fn pause_music(&mut self) {
        if let Some(ref music_sink) = self.music_sink {
            music_sink.pause();
        }
    }

    pub fn resume_music(&mut self) {
        if let Some(ref music_sink) = self.music_sink {
            music_sink.play();
        }
    }

    // Volume controls
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
        self.update_all_volumes();
    }

    pub fn set_sound_volume(&mut self, volume: f32) {
        self.sound_volume = volume.clamp(0.0, 1.0);
        self.update_sound_volumes();
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        self.update_music_volume();
    }

    pub fn get_master_volume(&self) -> f32 {
        self.master_volume
    }

    pub fn get_sound_volume(&self) -> f32 {
        self.sound_volume
    }

    pub fn get_music_volume(&self) -> f32 {
        self.music_volume
    }

    fn update_all_volumes(&self) {
        self.update_sound_volumes();
        self.update_music_volume();
    }

    fn update_sound_volumes(&self) {
        let volume = self.master_volume * self.sound_volume;
        for sink in self.sound_sinks.values() {
            sink.set_volume(volume);
        }
    }

    fn update_music_volume(&self) {
        if let Some(ref music_sink) = self.music_sink {
            music_sink.set_volume(self.master_volume * self.music_volume);
        }
    }

    pub fn is_sound_playing(&self, sound_id: &str) -> bool {
        self.sound_sinks.get(sound_id)
            .map(|sink| !sink.is_paused() && !sink.empty())
            .unwrap_or(false)
    }

    pub fn is_music_playing(&self) -> bool {
        self.music_sink.as_ref()
            .map(|sink| !sink.is_paused() && !sink.empty())
            .unwrap_or(false)
    }

    pub fn cleanup_finished_sounds(&mut self) {
        self.sound_sinks.retain(|_, sink| !sink.empty());
    }

    pub fn get_active_sound_count(&self) -> usize {
        self.sound_sinks.len()
    }
}