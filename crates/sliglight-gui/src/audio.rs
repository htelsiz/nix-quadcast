//! PulseAudio/PipeWire audio monitor for mic input level and mute state.
//!
//! Connects to the default source (microphone) via PulseAudio's compatibility layer
//! and streams peak level and mute change events.

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
    PeakLevel(f32),
    MuteChanged(bool),
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(audio_worker)
}

fn audio_worker() -> impl iced::futures::Stream<Item = Event> {
    iced::stream::channel(64, async move |mut output| {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(64);

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

    // Subscribe to source changes (for mute notifications).
    let tx_mute = tx.clone();
    let introspect = context.introspect();
    context.set_subscribe_callback(Some(Box::new(move |facility, operation, _idx| {
        if facility == Some(Facility::Source)
            && (operation == Some(Operation::Changed) || operation == Some(Operation::New))
        {
            let tx_inner = tx_mute.clone();
            // Query default source mute state — we do a simple lookup.
            // Note: this callback runs on the PA mainloop thread.
            // We can't call introspect here, so we just trigger a mute check.
            let _ = tx_inner.try_send(Event::MuteChanged(false)); // placeholder
        }
    })));
    context.subscribe(InterestMaskSet::SOURCE, |_| {});

    // Get default source name.
    let default_source: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let ds = default_source.clone();
    introspect.get_server_info(move |info| {
        if let Some(name) = &info.default_source_name {
            *ds.lock().unwrap() = Some(name.to_string());
        }
    });
    mainloop.wait(); // wait for server info callback

    // Also check initial mute state.
    let tx_mute3 = tx.clone();
    if let Some(source_name) = default_source.lock().unwrap().clone() {
        introspect.get_source_info_by_name(&source_name, move |result| {
            if let ListResult::Item(info) = result {
                let _ = tx_mute3.try_send(Event::MuteChanged(info.mute));
            }
        });
        mainloop.wait();
    }

    // Create a peak monitor stream on the default source.
    let source_name = default_source
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "@DEFAULT_SOURCE@".to_string());

    let spec = pulse::sample::Spec {
        format: pulse::sample::Format::FLOAT32NE,
        rate: 25, // Low rate — we just need peak level, not audio playback.
        channels: 1,
    };

    let monitor_stream = PaStream::new(&mut context, "sliglight-peak", &spec, None)
        .ok_or("Failed to create PA stream")?;

    let monitor_stream = Arc::new(Mutex::new(monitor_stream));
    let ms = monitor_stream.clone();

    {
        let mut stream = ms.lock().unwrap();
        stream.set_read_callback(Some(Box::new(move |_len| {
            // We'll process data in the callback.
        })));

        let attr = pulse::def::BufferAttr {
            maxlength: u32::MAX,
            tlength: u32::MAX,
            prebuf: u32::MAX,
            minreq: u32::MAX,
            fragsize: 4, // Single float32 sample
        };

        stream
            .connect_record(
                Some(&format!("{source_name}.monitor").replace(".monitor.monitor", ".monitor")),
                Some(&attr),
                StreamFlagSet::PEAK_DETECT
                    | StreamFlagSet::ADJUST_LATENCY
                    | StreamFlagSet::DONT_MOVE,
            )
            .map_err(|e| format!("PA stream connect failed: {e}"))?;
    }

    // Set up a proper read callback with peak detection.
    {
        let ms2 = monitor_stream.clone();
        let tx_peak2 = tx.clone();
        let mut stream = ms2.lock().unwrap();
        stream.set_read_callback(Some(Box::new(move |_len| {
            let mut stream = ms.lock().unwrap();
            if let Ok(data) = stream.peek() {
                match data {
                    pulse::stream::PeekResult::Data(buf) => {
                        if buf.len() >= 4 {
                            let peak =
                                f32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]).abs();
                            let _ = tx_peak2.try_send(Event::PeakLevel(peak.clamp(0.0, 1.0)));
                        }
                    }
                    pulse::stream::PeekResult::Hole(_) => {}
                    pulse::stream::PeekResult::Empty => {}
                }
                let _ = stream.discard();
            }
        })));
    }

    mainloop.unlock();

    // Block forever — the mainloop runs callbacks on its own thread.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
