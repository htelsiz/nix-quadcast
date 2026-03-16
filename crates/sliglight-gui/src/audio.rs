//! PulseAudio/PipeWire audio monitor for mic input, desktop audio, and mute state.
//!
//! Two independent peak-detect streams:
//!   1. **Mic** — default source (microphone) for voice-reactive LEDs.
//!   2. **Music** — default sink monitor (speaker loopback) for music-reactive LEDs.
//!
//! Mute state is polled periodically rather than queried from inside PA callbacks
//! (calling introspect inside subscribe callbacks crashes libpulse-binding).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::futures::SinkExt;
use iced::Subscription;
use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::subscribe::{Facility, InterestMaskSet, Operation};
use pulse::context::{Context, FlagSet as CtxFlagSet};
use pulse::mainloop::threaded::Mainloop;
use pulse::proplist::Proplist;
use pulse::stream::{FlagSet as StreamFlagSet, Stream as PaStream};

#[derive(Debug, Clone)]
pub enum Event {
    /// Mic input peak level (0.0-1.0).
    MicPeakLevel(f32),
    /// Desktop audio output peak level (0.0-1.0).
    MusicPeakLevel(f32),
    /// Mic mute state changed.
    MuteChanged(bool),
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(audio_worker)
}

fn audio_worker() -> impl iced::futures::Stream<Item = Event> {
    iced::stream::channel(128, async move |mut output| {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(128);

        let tx_clone = tx.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = run_pa_monitor(tx_clone) {
                log::warn!("PulseAudio monitor failed: {e}");
            }
        });

        while let Some(event) = rx.recv().await {
            let _ = output.send(event).await;
        }
    })
}

/// Wait for a PA async operation by temporarily unlocking the mainloop.
/// The callback should send on `done_tx` when complete.
fn pa_wait(
    mainloop: &mut Mainloop,
    done_rx: &std::sync::mpsc::Receiver<()>,
) {
    mainloop.unlock();
    // Give the PA thread time to process. Timeout prevents deadlock.
    let _ = done_rx.recv_timeout(Duration::from_secs(5));
    mainloop.lock();
}

fn run_pa_monitor(tx: tokio::sync::mpsc::Sender<Event>) -> Result<(), String> {
    let mut proplist = Proplist::new().ok_or("Failed to create Proplist")?;
    let _ = proplist.set_str(
        pulse::proplist::properties::APPLICATION_NAME,
        "Sliglight",
    );

    let mut mainloop = Mainloop::new().ok_or("Failed to create PA mainloop")?;
    let mut context = Context::new_with_proplist(&mainloop, "sliglight", &proplist)
        .ok_or("Failed to create PA context")?;

    context
        .connect(None, CtxFlagSet::NOFLAGS, None)
        .map_err(|e| format!("PA connect failed: {e}"))?;

    mainloop.lock();
    mainloop
        .start()
        .map_err(|e| format!("PA mainloop start failed: {e}"))?;

    // Wait for context ready.
    loop {
        match context.get_state() {
            pulse::context::State::Ready => break,
            pulse::context::State::Failed | pulse::context::State::Terminated => {
                mainloop.unlock();
                return Err("PA context failed".into());
            }
            _ => {
                // Briefly unlock so PA thread can process state changes.
                mainloop.unlock();
                std::thread::sleep(Duration::from_millis(50));
                mainloop.lock();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Discover default source (mic) and default sink (speakers)
    // -----------------------------------------------------------------------
    let default_source: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let default_sink: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
        let ds = default_source.clone();
        let dk = default_sink.clone();
        context.introspect().get_server_info(move |info| {
            if let Some(name) = &info.default_source_name {
                *ds.lock().unwrap() = Some(name.to_string());
            }
            if let Some(name) = &info.default_sink_name {
                *dk.lock().unwrap() = Some(name.to_string());
            }
            let _ = done_tx.try_send(());
        });
        pa_wait(&mut mainloop, &done_rx);
    }

    let source_name = default_source
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "@DEFAULT_SOURCE@".to_string());

    let sink_name = default_sink
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "@DEFAULT_SINK@".to_string());

    log::info!("Mic source: {source_name}");
    log::info!("Music sink: {sink_name}");

    // -----------------------------------------------------------------------
    // Query initial mute state
    // -----------------------------------------------------------------------
    let last_mute = Arc::new(AtomicBool::new(false));
    {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
        let lm = last_mute.clone();
        let tx_init = tx.clone();
        context
            .introspect()
            .get_source_info_by_name(&source_name, move |result| {
                if let ListResult::Item(info) = result {
                    lm.store(info.mute, Ordering::Relaxed);
                    let _ = tx_init.try_send(Event::MuteChanged(info.mute));
                }
                let _ = done_tx.try_send(());
            });
        pa_wait(&mut mainloop, &done_rx);
    }

    // -----------------------------------------------------------------------
    // Subscribe to source changes — set a flag, don't call introspect from
    // inside callbacks (that crashes libpulse-binding).
    // -----------------------------------------------------------------------
    let mute_dirty = Arc::new(AtomicBool::new(false));
    {
        let dirty = mute_dirty.clone();
        context.set_subscribe_callback(Some(Box::new(move |facility, operation, _idx| {
            if facility == Some(Facility::Source)
                && (operation == Some(Operation::Changed)
                    || operation == Some(Operation::New))
            {
                dirty.store(true, Ordering::Relaxed);
            }
        })));
        context.subscribe(InterestMaskSet::SOURCE, |_| {});
    }

    // -----------------------------------------------------------------------
    // Peak-detect stream config
    // -----------------------------------------------------------------------
    let spec = pulse::sample::Spec {
        format: pulse::sample::Format::FLOAT32NE,
        rate: 25,
        channels: 1,
    };
    let attr = pulse::def::BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: 4,
    };

    // -----------------------------------------------------------------------
    // 1. Mic peak monitor — connect to source directly
    // -----------------------------------------------------------------------
    let mic_stream =
        PaStream::new(&mut context, "sliglight-mic-peak", &spec, None)
            .ok_or("Failed to create mic PA stream")?;
    let mic_stream = Arc::new(Mutex::new(mic_stream));
    {
        let ms_read = mic_stream.clone();
        let tx_mic = tx.clone();
        let mut stream = mic_stream.lock().unwrap();
        stream.set_read_callback(Some(Box::new(move |_len| {
            let mut s = ms_read.lock().unwrap();
            if let Ok(data) = s.peek() {
                match data {
                    pulse::stream::PeekResult::Data(buf) if buf.len() >= 4 => {
                        let peak =
                            f32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]).abs();
                        let _ = tx_mic.try_send(Event::MicPeakLevel(peak.clamp(0.0, 1.0)));
                    }
                    _ => {}
                }
                let _ = s.discard();
            }
        })));
        stream
            .connect_record(
                Some(&source_name),
                Some(&attr),
                StreamFlagSet::PEAK_DETECT
                    | StreamFlagSet::ADJUST_LATENCY
                    | StreamFlagSet::DONT_MOVE,
            )
            .map_err(|e| format!("Mic stream connect failed: {e}"))?;
    }

    // -----------------------------------------------------------------------
    // 2. Music peak monitor — connect to sink's .monitor
    // -----------------------------------------------------------------------
    let sink_monitor = format!("{sink_name}.monitor");
    log::info!("Music monitor source: {sink_monitor}");

    let music_stream =
        PaStream::new(&mut context, "sliglight-music-peak", &spec, None)
            .ok_or("Failed to create music PA stream")?;
    let music_stream = Arc::new(Mutex::new(music_stream));
    {
        let ms_read = music_stream.clone();
        let tx_music = tx.clone();
        let mut stream = music_stream.lock().unwrap();
        stream.set_read_callback(Some(Box::new(move |_len| {
            let mut s = ms_read.lock().unwrap();
            if let Ok(data) = s.peek() {
                match data {
                    pulse::stream::PeekResult::Data(buf) if buf.len() >= 4 => {
                        let peak =
                            f32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]).abs();
                        let _ =
                            tx_music.try_send(Event::MusicPeakLevel(peak.clamp(0.0, 1.0)));
                    }
                    _ => {}
                }
                let _ = s.discard();
            }
        })));
        stream
            .connect_record(
                Some(&sink_monitor),
                Some(&attr),
                StreamFlagSet::PEAK_DETECT
                    | StreamFlagSet::ADJUST_LATENCY
                    | StreamFlagSet::DONT_MOVE,
            )
            .map_err(|e| format!("Music stream connect failed: {e}"))?;
    }

    mainloop.unlock();

    // -----------------------------------------------------------------------
    // Poll loop: check mute_dirty flag every 500ms.
    // When dirty, re-lock mainloop, introspect source, send mute event.
    // -----------------------------------------------------------------------
    let source_for_poll = source_name.clone();
    loop {
        std::thread::sleep(Duration::from_millis(500));

        // Check if channel is closed (app shutting down).
        if tx.is_closed() {
            mainloop.lock();
            mainloop.stop();
            mainloop.unlock();
            return Ok(());
        }

        if mute_dirty.swap(false, Ordering::Relaxed) {
            mainloop.lock();
            let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
            let lm = last_mute.clone();
            let tx_mute = tx.clone();
            context
                .introspect()
                .get_source_info_by_name(&source_for_poll, move |result| {
                    if let ListResult::Item(info) = result {
                        let prev = lm.swap(info.mute, Ordering::Relaxed);
                        if prev != info.mute {
                            let _ = tx_mute.try_send(Event::MuteChanged(info.mute));
                        }
                    }
                    let _ = done_tx.try_send(());
                });
            pa_wait(&mut mainloop, &done_rx);
            mainloop.unlock();
        }
    }
}
