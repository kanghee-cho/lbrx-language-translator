use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField,
};
use rdev::{Button, EventType, Key};
use std::cell::RefCell;

#[derive(Default)]
struct Input {
    flags: CGEventFlags,
}

impl Input {
    fn dispatch(
        &mut self,
        event_type: CGEventType,
        code: i64,
        flags: CGEventFlags,
        repeat: bool,
        callback: &mut impl FnMut(EventType),
    ) {
        for (flag, key) in [
            (CGEventFlags::CGEventFlagCommand, Key::MetaLeft),
            (CGEventFlags::CGEventFlagControl, Key::ControlLeft),
            (CGEventFlags::CGEventFlagShift, Key::ShiftLeft),
            (CGEventFlags::CGEventFlagAlternate, Key::Alt),
        ] {
            let pressed = flags.contains(flag);
            if pressed != self.flags.contains(flag) {
                callback(if pressed {
                    EventType::KeyPress(key)
                } else {
                    EventType::KeyRelease(key)
                });
            }
        }
        self.flags = flags;
        match event_type {
            CGEventType::KeyDown | CGEventType::KeyUp => {
                if repeat {
                    return;
                }
                // Physical key codes avoid Carbon input-source APIs, which require the main queue.
                let key = match code {
                    8 => Key::KeyC,
                    53 => Key::Escape,
                    code => Key::Unknown(code as u32),
                };
                callback(if matches!(event_type, CGEventType::KeyDown) {
                    EventType::KeyPress(key)
                } else {
                    EventType::KeyRelease(key)
                });
            }
            CGEventType::FlagsChanged if !matches!(code, 54 | 55 | 56 | 58 | 59 | 60 | 61 | 62) => {
                callback(EventType::KeyPress(Key::Unknown(code as u32)));
            }
            CGEventType::LeftMouseDown => callback(EventType::ButtonPress(Button::Left)),
            CGEventType::RightMouseDown => callback(EventType::ButtonPress(Button::Right)),
            CGEventType::OtherMouseDown => callback(EventType::ButtonPress(Button::Middle)),
            _ => {}
        }
    }
}

pub(super) fn listen(callback: impl FnMut(EventType) + Send + 'static) -> Result<(), String> {
    let state = RefCell::new((Input::default(), callback));
    let tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
            CGEventType::LeftMouseDown,
            CGEventType::RightMouseDown,
            CGEventType::OtherMouseDown,
        ],
        move |_, event_type, event| {
            if matches!(
                event_type,
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput
            ) {
                CFRunLoop::get_current().stop();
                return CallbackResult::Keep;
            }
            let (input, callback) = &mut *state.borrow_mut();
            input.dispatch(
                event_type,
                event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE),
                event.get_flags(),
                event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0,
                callback,
            );
            CallbackResult::Keep
        },
    )
    .map_err(|()| "macOS 입력 모니터링 권한을 확인한 뒤 앱을 다시 실행해주세요.".to_owned())?;
    let source = tap
        .mach_port()
        .create_runloop_source(0)
        .map_err(|()| "macOS 입력 감지 실행 루프를 만들지 못했습니다.".to_owned())?;
    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
    tap.enable();
    CFRunLoop::run_current();
    Err("macOS 입력 감지가 중단되었습니다. 앱을 다시 실행해주세요.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::Detector;
    use std::time::{Duration, Instant};

    #[test]
    fn physical_double_copy_triggers_without_character_conversion() {
        let mut input = Input::default();
        let mut detector = Detector::default();
        let mut translations = Vec::new();
        let now = Instant::now();
        let mut callback = |event| {
            let (key, pressed) = match event {
                EventType::KeyPress(key) => (key, true),
                EventType::KeyRelease(key) => (key, false),
                _ => return,
            };
            if let Some(revision) = detector.key(key, pressed, now, 7, Duration::from_millis(500)) {
                translations.push(revision);
            }
        };
        let command = CGEventFlags::CGEventFlagCommand;
        for event_type in [
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::KeyDown,
        ] {
            input.dispatch(event_type, 8, command, false, &mut callback);
        }
        assert_eq!(translations, [7]);
    }

    #[test]
    fn command_handoff_and_missing_flags_changed_use_current_event_flags() {
        let mut input = Input::default();
        let mut events = Vec::new();
        let command = CGEventFlags::CGEventFlagCommand;
        for code in [55, 54, 55] {
            input.dispatch(
                CGEventType::FlagsChanged,
                code,
                command,
                false,
                &mut |event| {
                    events.push(event);
                },
            );
        }
        input.dispatch(
            CGEventType::KeyDown,
            8,
            CGEventFlags::CGEventFlagNull,
            false,
            &mut |event| events.push(event),
        );
        assert_eq!(
            events,
            [
                EventType::KeyPress(Key::MetaLeft),
                EventType::KeyRelease(Key::MetaLeft),
                EventType::KeyPress(Key::KeyC),
            ]
        );
    }

    #[test]
    fn repeats_are_ignored_and_other_keys_and_mouse_clicks_are_forwarded() {
        let mut input = Input::default();
        let mut events = Vec::new();
        for (event_type, code, repeat) in [
            (CGEventType::KeyDown, 8, true),
            (CGEventType::KeyDown, 7, false),
            (CGEventType::KeyDown, 53, false),
            (CGEventType::LeftMouseDown, 0, false),
        ] {
            input.dispatch(
                event_type,
                code,
                CGEventFlags::CGEventFlagNull,
                repeat,
                &mut |event| events.push(event),
            );
        }
        assert_eq!(
            events,
            [
                EventType::KeyPress(Key::Unknown(7)),
                EventType::KeyPress(Key::Escape),
                EventType::ButtonPress(Button::Left),
            ]
        );
    }
}
