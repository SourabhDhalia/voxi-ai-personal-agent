//! CPAL audio backend — real microphone capture and speaker playback.
//!
//! Built only under the `cpal-audio` feature. Capture downmixes to mono f32 at
//! the device's native rate and forwards fixed-size chunks; playback resamples
//! is intentionally omitted in Stage 1 (the device is opened at the requested
//! rate when supported, otherwise the default config is used).

use super::{AudioBackend, AudioError, AudioFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

/// Number of mono samples per emitted capture chunk (~10 ms at 16 kHz).
const CHUNK_SAMPLES: usize = 160;

pub struct CpalBackend {
    host_id: String,
    /// Live capture stream kept alive for the backend's lifetime.
    capture_stream: Mutex<Option<cpal::Stream>>,
}

// cpal::Stream is not Sync; we guard it behind a Mutex and never share the
// stream itself across threads, so the backend is safe to share.
unsafe impl Sync for CpalBackend {}
unsafe impl Send for CpalBackend {}

impl CpalBackend {
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let _ = host
            .default_input_device()
            .ok_or_else(|| AudioError::NoDevice("no default input device".into()))?;
        Ok(CpalBackend {
            host_id: format!("{:?}", host.id()),
            capture_stream: Mutex::new(None),
        })
    }
}

impl AudioBackend for CpalBackend {
    fn name(&self) -> &str {
        "cpal"
    }

    fn start_capture(&self, _format: AudioFormat) -> Result<Receiver<Vec<f32>>, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AudioError::NoDevice("no default input device".into()))?;
        let config = device
            .default_input_config()
            .map_err(|e| AudioError::Backend(e.to_string()))?;
        let channels = config.channels() as usize;

        let (tx, rx) = channel::<Vec<f32>>();
        let acc = Arc::new(Mutex::new(Vec::<f32>::with_capacity(CHUNK_SAMPLES * 2)));
        let acc_cb = acc.clone();

        let err_fn = |e| log::warn!("CPAL capture stream error: {e}");

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = match acc_cb.lock() {
                        Ok(g) => g,
                        Err(_) => return,
                    };
                    // Downmix interleaved frames to mono by averaging channels.
                    for frame in data.chunks(channels.max(1)) {
                        let sum: f32 = frame.iter().copied().sum();
                        buf.push(sum / channels.max(1) as f32);
                        if buf.len() >= CHUNK_SAMPLES {
                            let chunk = std::mem::take(&mut *buf);
                            let _ = tx.send(chunk);
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::Backend(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::Backend(e.to_string()))?;

        if let Ok(mut guard) = self.capture_stream.lock() {
            *guard = Some(stream);
        }
        log::info!("CpalBackend: capture started on host {}", self.host_id);
        Ok(rx)
    }

    fn play(&self, pcm: &[f32], _format: AudioFormat) -> Result<(), AudioError> {
        use cpal::traits::DeviceTrait;
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::NoDevice("no default output device".into()))?;
        let config = device
            .default_output_config()
            .map_err(|e| AudioError::Backend(e.to_string()))?;
        let channels = config.channels() as usize;

        let samples: Arc<Vec<f32>> = Arc::new(pcm.to_vec());
        let pos = Arc::new(Mutex::new(0usize));
        let done = Arc::new((Mutex::new(false), std::sync::Condvar::new()));

        let samples_cb = samples.clone();
        let pos_cb = pos.clone();
        let done_cb = done.clone();
        let err_fn = |e| log::warn!("CPAL playback stream error: {e}");

        let stream = device
            .build_output_stream(
                &config.into(),
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut p = pos_cb.lock().unwrap();
                    for frame in out.chunks_mut(channels.max(1)) {
                        let s = samples_cb.get(*p).copied().unwrap_or(0.0);
                        for ch in frame.iter_mut() {
                            *ch = s;
                        }
                        if *p < samples_cb.len() {
                            *p += 1;
                        }
                    }
                    if *p >= samples_cb.len() {
                        let (lock, cvar) = &*done_cb;
                        if let Ok(mut d) = lock.lock() {
                            *d = true;
                            cvar.notify_all();
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::Backend(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::Backend(e.to_string()))?;

        // Block until the buffer has been fully emitted.
        let (lock, cvar) = &*done;
        let mut d = lock.lock().unwrap();
        while !*d {
            let (guard, timeout) = cvar
                .wait_timeout(d, std::time::Duration::from_secs(30))
                .unwrap();
            d = guard;
            if timeout.timed_out() {
                break;
            }
        }
        Ok(())
    }
}
