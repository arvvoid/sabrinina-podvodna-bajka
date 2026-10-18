// SPDX-License-Identifier: MIT
// The melody generator retains the original waveform constants.
#![allow(clippy::approx_constant)]
use crate::{
    app::{FrameEvents, Screen, SessionUi, Simulation},
    settings::Preferences,
    ui::tr,
};
use bevy::{audio::Volume, prelude::*};
#[derive(Resource, Default)]
pub struct AudioRequests {
    pub help: Option<usize>,
    pub stop_voice: bool,
    pub pause_voice: bool,
}
#[derive(Component, Clone, Copy, PartialEq)]
enum Channel {
    Calm,
    Visit,
    Song,
    Effect,
    Voice,
}
pub struct ReefAudioPlugin;
impl Plugin for ReefAudioPlugin {
    fn build(&self, a: &mut App) {
        a.init_resource::<AudioRequests>()
            .add_systems(Startup, setup)
            .add_systems(Update, update);
    }
}
fn compose(visit: bool) -> AudioSource {
    let rate = 22050_u32;
    let beat = if visit { 0.45_f32 } else { 0.65 };
    let length = (rate as f32 * beat * 64.0) as u32;
    let mut data = Vec::with_capacity(length as usize * 2 + 44);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + length * 2).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16_u32.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&1_u16.to_le_bytes());
    data.extend_from_slice(&rate.to_le_bytes());
    data.extend_from_slice(&(rate * 2).to_le_bytes());
    data.extend_from_slice(&2_u16.to_le_bytes());
    data.extend_from_slice(&16_u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&(length * 2).to_le_bytes());
    let melody = [
        76, 0, 79, 0, 81, 79, 76, 0, 74, 0, 72, 0, 74, 76, 79, 0, 76, 0, 72, 0, 69, 72, 76, 0, 74,
        0, 76, 79, 74, 0, 72, 0,
    ];
    let roots = [48, 45, 41, 43];
    for n in 0..length {
        let t = n as f32 / rate as f32;
        let step = (t / beat) as usize;
        let bar = step / 16;
        let local = t - step as f32 * beat;
        let mut sample = 0.0;
        let note = melody[step % 32];
        if note != 0 {
            let hz = 440.0 * 2_f32.powf((note - 69) as f32 / 12.0);
            let envelope = (local / 0.015).min(1.0) * (-local * 5.0).exp();
            sample += 0.25
                * envelope
                * ((6.283185 * hz * local).sin() + 0.2 * (6.283185 * hz * 2.0 * local).sin());
        }
        let btime = t - bar as f32 * 16.0 * beat;
        let fade = (btime / 0.6).min(1.0).min((16.0 * beat - btime) / 0.8);
        for offset in [0, 7, 16] {
            let hz = 440.0 * 2_f32.powf((roots[bar % 4] + offset - 69) as f32 / 12.0);
            sample += 0.055 * fade * (6.283185 * hz * btime).sin();
        }
        if visit {
            sample += 0.025 * (local * 6.283185 * 110.0).sin() * (-local * 18.0).exp();
        }
        data.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 22000.0) as i16).to_le_bytes());
    }
    AudioSource { bytes: data.into() }
}
fn setup(mut c: Commands, mut assets: ResMut<Assets<AudioSource>>, server: Res<AssetServer>) {
    for (channel, source) in [
        (Channel::Calm, assets.add(compose(false))),
        (Channel::Visit, assets.add(compose(true))),
        (Channel::Song, server.load("audio/siren_song.ogg")),
    ] {
        c.spawn((
            channel,
            AudioPlayer::new(source),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
        ));
    }
}
fn update(
    mut c: Commands,
    server: Res<AssetServer>,
    mut requests: ResMut<AudioRequests>,
    mut events: ResMut<FrameEvents>,
    w: Res<Simulation>,
    mut u: ResMut<SessionUi>,
    p: Res<Preferences>,
    time: Res<Time>,
    mut channels: Query<(Entity, &Channel, Option<&mut AudioSink>)>,
) {
    let mut voice_playing = channels
        .iter()
        .any(|(_, channel, sink)| *channel == Channel::Voice && sink.is_none_or(|s| !s.empty()));
    if requests.stop_voice {
        for (e, ch, _) in &channels {
            if *ch == Channel::Voice {
                c.entity(e).despawn();
            }
        }
        u.caption.clear();
        voice_playing = false;
        requests.stop_voice = false;
    }
    if requests.pause_voice {
        for (_, ch, sink) in &channels {
            if *ch == Channel::Voice
                && let Some(sink) = sink
            {
                sink.toggle_playback()
            }
        }
        requests.pause_voice = false;
    }
    if let Some(lang) = requests.help.take() {
        for (e, ch, _) in &channels {
            if *ch == Channel::Voice {
                c.entity(e).despawn();
            }
        }
        c.spawn((
            Channel::Voice,
            AudioPlayer::new(server.load(format!("audio/help_{}.ogg", ["en", "hr", "it"][lang]))),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(p.voice)),
        ));
        voice_playing = true;
    }
    let mut effect = None;
    let mut voice = None;
    for e in events.0.drain(..) {
        if e.song_empty {
            u.notice = tr("The song is resting. It must recharge fully.", p.language);
            u.notice_time = 4.0;
        }
        if e.wave_cast {
            effect = Some(("magic_burst", 0.65))
        }
        if e.cast {
            effect = Some(("magic_charge", 1.0))
        }
        if e.teleported > 0 {
            effect = Some(("magic_collect", 1.1))
        }
        if e.repelled > 0 {
            effect = Some(("magic_burst", 1.0));
            voice = Some(("jelly_repelled", "That mermaid and her crystal!"));
        }
        if e.hit {
            effect = Some(("sting", 1.0));
            u.notice = tr("A little sting. Sabrina will recover soon.", p.language);
            u.notice_time = 4.0;
        }
        if e.visit_started {
            voice = Some(("jelly_arrive", "I'm hungry! Where are my little fish?"));
        }
        if e.eaten > 0 {
            voice = Some(("jelly_eat", "Oops! I ate you!"));
        }
        if e.leaving {
            voice = Some(("jelly_leave", "The jellyfish is swimming away."));
        }
        if e.game_over {
            requests.stop_voice = true;
            u.notice_time = 0.0;
            u.score_time = 0.0;
        }
    }
    if let Some((name, speed)) = effect {
        for (e, ch, _) in &channels {
            if *ch == Channel::Effect {
                c.entity(e).despawn();
            }
        }
        c.spawn((
            Channel::Effect,
            AudioPlayer::new(server.load(format!("audio/{name}.ogg"))),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(p.effects))
                .with_speed(speed),
        ));
    }
    if let Some((name, caption)) = voice
        && !voice_playing
    {
        u.caption = tr(caption, p.language);
        c.spawn((
            Channel::Voice,
            AudioPlayer::new(server.load(format!("audio/{name}.ogg"))),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(p.voice)),
        ));
        voice_playing = true;
    }
    let duck = if voice_playing { 0.25 } else { 1.0 } * (1.0 - w.0.sabrina.song.strength * 0.60);
    let visit = u.started && !w.0.session.game_over && w.0.session.water == reef_sim::Water::Visit;
    for (_, channel, sink) in &mut channels {
        let Some(mut sink) = sink else { continue };
        let target = match channel {
            Channel::Calm => p.music * duck * if visit { 0.12 } else { 1.0 },
            Channel::Visit => p.music * duck * if visit { 1.0 } else { 0.0 },
            Channel::Song => p.effects * 0.65 * w.0.sabrina.song.strength,
            Channel::Effect => p.effects,
            Channel::Voice => p.voice,
        };
        if matches!(channel, Channel::Calm | Channel::Visit) {
            let volume = sink.volume().to_linear();
            sink.set_volume(Volume::Linear(
                volume + (target - volume) * (time.delta_secs() * 2.0).min(1.0),
            ));
        } else {
            sink.set_volume(Volume::Linear(target));
        }
        if *channel == Channel::Song {
            if w.0.sabrina.song.strength > 0.01 && u.screen == Screen::Playing {
                sink.play();
            } else {
                if !sink.is_paused() {
                    sink.pause();
                    let _ = sink.try_seek(std::time::Duration::ZERO);
                }
            }
        }
    }
    if !voice_playing {
        u.caption.clear();
    }
}
