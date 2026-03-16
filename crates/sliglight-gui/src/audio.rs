//! PulseAudio/PipeWire audio monitor for mic input, desktop audio, and mute state.
//!
//! Two independent monitors:
//!   1. **Mic** — connects to the default *source* (microphone) for voice-reactive LEDs.
//!   2. **Music** — connects to the default *sink monitor* (speaker loopback) for music-reactive LEDs.
//!
//! Both use PulseAudio PEAK_DETECT streams so we never actually buffer audio data.

use std::sync::{Arc, Mutex};

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
    /// Mic input peak level (0.0–1.0).
    MicPeakLevel(f32),
    /// Desktop audio output peak level (0.0–1.0).
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

        // PulseAudio mainloop must run on a dedicated thread.
        let tx_clone = tx.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = run_pa_monitor(tx_clone) {
                log::warn!("PulseAudio monitor failed: {e}");
            }
        });

        // Forward events from the PA thread to the iced stream.
        while let Some(event) = rx.recv().await {
            let _ = output.send(event).await;
        }
    })
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
    mainloop.start().map_err(|e| format!("PA mainloop start failed: {e}"))?;

    // Wait for context to be ready.
    loop {
        match context.get_state() {
            pulse::context::State::Ready => break,
            pulse::context::State::Failed | pulse::context::State::Terminated => {
                mainloop.unlock();
                return Err("PA context failed".into());
            }
            _ => {
                mainloop.wait();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Discover default source (mic) and default sink (speakers)
    // -----------------------------------------------------------------------
    let default_source: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let default_sink: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    {
        let ds = default_source.clone();
        let dk = default_sink.clone();
        let introspect = context.introspect();
        introspect.get_server_info(move |info| {
            if let Some(name) = &info.default_source_name {
                *ds.lock().unwrap() = Some(name.to_string());
            }
            if let Some(name) = &info.default_sink_name {
                *dk.lock().unwrap() = Some(name.to_string());
            }
        });
        mainloop.wait(); // wait for server info callback
    }

    // -----------------------------------------------------------------------
    // Query initial mute state of default source
    // -----------------------------------------------------------------------
    let introspect = context.introspect();
    {
        let tx_mute_init = tx.clone();
        if let Some(ref source_name) = *default_source.lock().unwrap() {
            introspect.get_source_info_by_name(source_name, move |result| {
                if let ListResult::Item(info) = result {
                    let _ = tx_mute_init.try_send(Event::MuteChanged(info.mute));
                }
            });
            mainloop.wait();
        }
    }

    // -----------------------------------------------------------------------
    // Subscribe to source changes for mute notifications
    // -----------------------------------------------------------------------
    {
        let tx_mute = tx.clone();
        let ds_for_cb = default_source.clone();
        let intro = context.introspect();
        context.set_subscribe_callback(Some(Box::new(move |facility, operation, _idx| {
            if facility == Some(Facility::Source)
                && (operation == Some(Operation::Changed) || operation == Some(Operation::New))
            {
                // Re-query default source mute state.
                if let Some(ref source_name) = *ds_for_cb.lock().unwrap() {
                    let tx_cb = tx_mute.clone();
                    intro.get_source_info_by_name(source_name, move |result| {
                        if let ListResult::Item(info) = result {
                            let _ = tx_cb.try_send(Event::MuteChanged(info.mute));
                        }
                    });
                }
            }
        })));
        context.subscribe(InterestMaskSet::SOURCE, |_| {});
    }

    // -----------------------------------------------------------------------
    // Peak-detect stream helper
    // -----------------------------------------------------------------------
    let spec = pulse::sample::Spec {
        format: pulse::sample::Format::FLOAT32NE,
        rate: 25, // Low rate — we just need peak level, not audio playback.
        channels: 1,
    };
    let attr = pulse::def::BufferAttr {
        maxlength: u32::MAX,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: 4, // Single float32 sample per read
    };

    // -----------------------------------------------------------------------
    // 1. Mic peak monitor — connect to default SOURCE directly (no .monitor)
    // -----------------------------------------------------------------------
    let source_name = default_source
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "@DEFAULT_SOURCE@".to_string());

    log::info!("Mic peak monitor connecting to source: {source_name}");

    let mic_stream = PaStream::new(&mut context, "sliglight-mic-peak", &spec, None)
        .ok_or("Failed to create mic PA stream")?;
    let mic_stream = Arc::new(Mutex::new(mic_stream));

    {
        let ms_read = mic_stream.clone();
        let tx_mic = tx.clone();
        let mut stream = mic_stream.lock().unwrap();

        // Set up read callback that extracts peak.
        stream.set_read_callback(Some(Box::new(move |_len| {
            let mut s = ms_read.lock().unwrap();
            if let Ok(data) = s.peek() {
                match data {
                    pulse::stream::PeekResult::Data(buf) => {
                        if buf.len() >= 4 {
                            let peak =
                                f32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]).abs();
                            let _ =
                                tx_mic.try_send(Event::MicPeakLevel(peak.clamp(0.0, 1.0)));
                        }
                    }
                    pulse::stream::PeekResult::Hole(_) | pulse::stream::PeekResult::Empty => {}
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
    // 2. Music peak monitor — connect to default SINK's .monitor
    //    A sink's monitor source captures the mixed audio output (music, games, etc.)
    // -----------------------------------------------------------------------
    let sink_monitor = {
        let sink_name = default_sink
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "@DEFAULT_SINK@".to_string());
        // The monitor source for a sink is always "<sink_name>.monitor"
        format!("{sink_name}.monitor")
    };

    log::info!("Music peak monitor connecting to sink monitor: {sink_monitor}");

    let music_stream = PaStream::new(&mut context, "sliglight-music-peak", &spec, None)
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
                    pulse::stream::PeekResult::Data(buf) => {
                        if buf.len() >= 4 {
                            let peak =
                                f32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]).abs();
                            let _ =
                                tx_music.try_send(Event::MusicPeakLevel(peak.clamp(0.0, 1.0)));
                        }
                    }
                    pulse::stream::PeekResult::Hole(_) | pulse::stream::PeekResult::Empty => {}
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

    // Block forever — the mainloop runs callbacks on its own thread.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
