use rdev::{EventType, Key};
use std::{
    sync::{mpsc::SyncSender, Arc, RwLock},
    time::{Duration, Instant},
};

use crate::config::Config;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos::listen;

#[cfg(not(target_os = "macos"))]
fn listen(callback: impl FnMut(EventType) + Send + 'static) -> Result<(), String> {
    let mut callback = callback;
    rdev::listen(move |event| callback(event.event_type)).map_err(|error| format!("{error:?}"))
}

pub enum Shortcut {
    Translate { clipboard_revision: u64 },
    Dismiss,
    Error(String),
}

#[derive(Default)]
struct Detector {
    modifiers: [bool; 2],
    blocked: [bool; 6],
    c_down: bool,
    first: Option<(Instant, u64)>,
}

impl Detector {
    fn key(
        &mut self,
        key: Key,
        pressed: bool,
        now: Instant,
        revision: u64,
        interval: Duration,
    ) -> Option<u64> {
        let modifiers = if cfg!(target_os = "macos") {
            [Key::MetaLeft, Key::MetaRight]
        } else {
            [Key::ControlLeft, Key::ControlRight]
        };
        let other_modifiers = if cfg!(target_os = "macos") {
            [
                Key::ControlLeft,
                Key::ControlRight,
                Key::ShiftLeft,
                Key::ShiftRight,
                Key::Alt,
                Key::AltGr,
            ]
        } else {
            [
                Key::MetaLeft,
                Key::MetaRight,
                Key::ShiftLeft,
                Key::ShiftRight,
                Key::Alt,
                Key::AltGr,
            ]
        };
        if let Some(index) = modifiers.iter().position(|item| *item == key) {
            self.modifiers[index] = pressed;
            if !self.modifiers.iter().any(|value| *value) {
                self.first = None;
            }
            return None;
        }
        if let Some(index) = other_modifiers.iter().position(|item| *item == key) {
            self.blocked[index] = pressed;
            self.first = None;
            return None;
        }
        if key != Key::KeyC {
            if pressed {
                self.first = None;
            }
            return None;
        }
        if !pressed {
            self.c_down = false;
            return None;
        }
        if self.c_down {
            return None;
        }
        self.c_down = true;
        if !self.modifiers.iter().any(|value| *value) || self.blocked.iter().any(|value| *value) {
            self.first = None;
            return None;
        }
        if let Some((first, baseline)) = self.first.take() {
            if now.duration_since(first) <= interval {
                return Some(baseline);
            }
        }
        self.first = Some((now, revision));
        None
    }
}

pub fn start(config: Arc<RwLock<Config>>, sender: SyncSender<Shortcut>) {
    std::thread::spawn(move || {
        let mut detector = Detector::default();
        let listener_sender = sender.clone();
        let result = listen(move |event| {
            let (key, pressed) = match event {
                EventType::KeyPress(key) => (key, true),
                EventType::KeyRelease(key) => (key, false),
                EventType::ButtonPress(_) => {
                    detector.first = None;
                    return;
                }
                _ => return,
            };
            if key == Key::Escape && pressed {
                let _ = listener_sender.try_send(Shortcut::Dismiss);
            }
            let interval = match config.read() {
                Ok(config) if config.consent => Duration::from_millis(config.double_copy_ms),
                Ok(_) => {
                    detector = Detector::default();
                    return;
                }
                Err(error) => {
                    eprintln!("Shortcut configuration lock failed: {error}");
                    return;
                }
            };
            let revision = if key == Key::KeyC && pressed && !detector.c_down {
                crate::platform::clipboard_revision()
            } else {
                0
            };
            if let Some(clipboard_revision) =
                detector.key(key, pressed, Instant::now(), revision, interval)
            {
                // Never block the OS input hook or suppress the original copy shortcut.
                let _ = listener_sender.try_send(Shortcut::Translate { clipboard_revision });
            }
        });
        if let Err(error) = result {
            let _ = sender.send(Shortcut::Error(format!("전역 단축키 감지 오류: {error}")));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifier() -> Key {
        if cfg!(target_os = "macos") {
            Key::MetaLeft
        } else {
            Key::ControlLeft
        }
    }

    #[test]
    fn double_copy_requires_release_and_preserves_initial_revision() {
        let mut detector = Detector::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        detector.key(modifier(), true, now, 0, interval);
        assert_eq!(detector.key(Key::KeyC, true, now, 7, interval), None);
        assert_eq!(detector.key(Key::KeyC, true, now, 8, interval), None);
        detector.key(Key::KeyC, false, now, 8, interval);
        assert_eq!(
            detector.key(
                Key::KeyC,
                true,
                now + Duration::from_millis(200),
                9,
                interval
            ),
            Some(7)
        );
    }

    #[test]
    fn expired_copy_and_intervening_keys_do_not_trigger() {
        let mut detector = Detector::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        detector.key(modifier(), true, now, 0, interval);
        detector.key(Key::KeyC, true, now, 1, interval);
        detector.key(Key::KeyC, false, now, 1, interval);
        let later = now + Duration::from_secs(1);
        assert_eq!(detector.key(Key::KeyC, true, later, 2, interval), None);
        detector.key(Key::KeyC, false, later, 2, interval);
        detector.key(Key::KeyX, true, later, 2, interval);
        assert_eq!(detector.key(Key::KeyC, true, later, 2, interval), None);
    }

    #[test]
    fn shift_and_released_modifier_prevent_translation() {
        let mut detector = Detector::default();
        let now = Instant::now();
        let interval = Duration::from_millis(500);
        detector.key(modifier(), true, now, 0, interval);
        detector.key(Key::ShiftLeft, true, now, 0, interval);
        detector.key(Key::KeyC, true, now, 1, interval);
        detector.key(Key::KeyC, false, now, 1, interval);
        assert_eq!(detector.key(Key::KeyC, true, now, 2, interval), None);
        detector.key(Key::ShiftLeft, false, now, 2, interval);
        detector.key(Key::KeyC, false, now, 2, interval);
        detector.key(Key::KeyC, true, now, 2, interval);
        detector.key(modifier(), false, now, 2, interval);
        detector.key(Key::KeyC, false, now, 2, interval);
        assert_eq!(detector.key(Key::KeyC, true, now, 2, interval), None);
    }
}
