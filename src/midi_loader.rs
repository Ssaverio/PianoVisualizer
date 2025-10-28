// src/midi_loader.rs
use midly::{Smf, TrackEventKind};
use std::collections::HashMap;

// Una struct per contenere i dati puliti estratti dal MIDI
#[derive(Debug, Clone)]
pub struct MidiNote {
    pub pitch: u8,
    pub velocity: u8,
    pub start_time_secs: f32,
    pub duration_secs: f32,
}

// Converte i "tick" del MIDI in secondi
fn ticks_to_secs(ticks: u32, ticks_per_beat: u16, microsecs_per_beat: u32) -> f32 {
    ((ticks as f64 * (microsecs_per_beat as f64 / 1_000_000.0)) / ticks_per_beat as f64) as f32
}

pub fn load_midi_file(path: &std::path::Path) -> Vec<MidiNote> {
    // Carica i byte del file
    let data = std::fs::read(path).expect("Impossibile leggere il file MIDI");
    let smf = Smf::parse(&data).expect("Impossibile parsare il file MIDI");

    let mut notes = Vec::new();

    // 1. Estrai l'impostazione "ticks per beat" (Tpq) dall'header
    let ticks_per_beat = match smf.header.timing {
        midly::Timing::Metrical(tpq) => tpq.as_int(),
        _ => 480, // Fallback comune
    };

    // 2. Trova tutti gli eventi di cambio Tempo (BPM)
    // Il MIDI memorizza il tempo come "microsecondi per beat"
    let mut tempo_changes = Vec::new();
    for event in &smf.tracks[0] {
        if let TrackEventKind::Meta(midly::MetaMessage::Tempo(us_per_beat)) = event.kind {
            tempo_changes.push((event.delta.as_int(), us_per_beat.as_int()));
        }
    }
    // Se non ci sono cambi di tempo, usa il default MIDI (120 BPM)
    if tempo_changes.is_empty() {
        tempo_changes.push((0, 500_000)); // 120 BPM = 500,000 µs per beat
    }

    // 3. Itera su tutte le tracce per trovare le note
    let mut current_ticks_total: u32 = 0;
    let mut current_us_per_beat: u32 = tempo_changes[0].1;
    let mut tempo_iter = tempo_changes.iter().peekable();

    // Mappa per tenere traccia delle note "NoteOn" in attesa del loro "NoteOff"
    // Key = (channel, pitch), Value = (start_tick, velocity)
    let mut pending_notes: HashMap<(u8, u8), (u32, u8)> = HashMap::new();

    for track in &smf.tracks {
        current_ticks_total = 0;

        for event in track {
            current_ticks_total += event.delta.as_int();

            // Aggiorna il tempo (BPM) se necessario
            if let Some((delta, us_per_beat)) = tempo_iter.peek() {
                if current_ticks_total >= *delta {
                    current_us_per_beat = *us_per_beat;
                    tempo_iter.next();
                }
            }

            if let TrackEventKind::Midi { channel, message } = event.kind {
                match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        let pitch = key.as_int();
                        let velocity = vel.as_int();

                        if velocity > 0 {
                            // Una vera "NoteOn"
                            if let Some((start_tick, old_vel)) =
                                pending_notes.remove(&(channel.as_int(), pitch))
                            {
                                let duration_ticks = current_ticks_total - start_tick;
                                notes.push(MidiNote {
                                    pitch,
                                    velocity: old_vel,
                                    start_time_secs: ticks_to_secs(
                                        start_tick,
                                        ticks_per_beat,
                                        current_us_per_beat,
                                    ),
                                    duration_secs: ticks_to_secs(
                                        duration_ticks,
                                        ticks_per_beat,
                                        current_us_per_beat,
                                    ),
                                });
                            }
                            pending_notes
                                .insert((channel.as_int(), pitch), (current_ticks_total, velocity));
                        } else {
                            // "NoteOn" con velocity 0 è una "NoteOff"
                            if let Some((start_tick, velocity)) =
                                pending_notes.remove(&(channel.as_int(), pitch))
                            {
                                let duration_ticks = current_ticks_total - start_tick;
                                notes.push(MidiNote {
                                    pitch,
                                    velocity,
                                    start_time_secs: ticks_to_secs(
                                        start_tick,
                                        ticks_per_beat,
                                        current_us_per_beat,
                                    ),
                                    duration_secs: ticks_to_secs(
                                        duration_ticks,
                                        ticks_per_beat,
                                        current_us_per_beat,
                                    ),
                                });
                            }
                        }
                    }
                    midly::MidiMessage::NoteOff { key, .. } => {
                        // Una "NoteOff"
                        let pitch = key.as_int();
                        if let Some((start_tick, velocity)) =
                            pending_notes.remove(&(channel.as_int(), pitch))
                        {
                            let duration_ticks = current_ticks_total - start_tick;
                            notes.push(MidiNote {
                                pitch,
                                velocity,
                                start_time_secs: ticks_to_secs(
                                    start_tick,
                                    ticks_per_beat,
                                    current_us_per_beat,
                                ),
                                duration_secs: ticks_to_secs(
                                    duration_ticks,
                                    ticks_per_beat,
                                    current_us_per_beat,
                                ),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Ordina le note per tempo di inizio
    notes.sort_by(|a, b| a.start_time_secs.partial_cmp(&b.start_time_secs).unwrap());
    notes
}
