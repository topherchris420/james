//! Opt-in global shortcuts, registered and released on the native UI thread.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceShortcutKind {
    Talk,
    Dictate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutEvent {
    Pressed,
    Released,
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use native::{SummonShortcut, VoiceShortcut};

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod native {
    use std::collections::VecDeque;
    use std::marker::PhantomData;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, Once, Weak};

    use super::{ShortcutEvent, VoiceShortcutKind};
    use eframe::egui;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

    // One process-wide handler fans events out to independent registrations.
    // Weak references ensure disabling a shortcut releases its subscriber.
    static INSTALL_HANDLER: Once = Once::new();
    static SUBSCRIBERS: Mutex<Vec<Weak<Signal>>> = Mutex::new(Vec::new());
    const MAX_PENDING_EVENTS: usize = 32;

    #[derive(Default)]
    struct EventState {
        held: bool,
        pending: VecDeque<ShortcutEvent>,
    }

    impl EventState {
        fn receive(&mut self, event: HotKeyState) -> bool {
            let next = match event {
                HotKeyState::Pressed if !self.held => {
                    self.held = true;
                    ShortcutEvent::Pressed
                }
                HotKeyState::Released if self.held => {
                    self.held = false;
                    ShortcutEvent::Released
                }
                _ => return false,
            };
            // Always keep the newest complete state when the GUI has stalled.
            if self.pending.len() == MAX_PENDING_EVENTS {
                self.pending.pop_front();
            }
            self.pending.push_back(next);
            true
        }
    }

    struct Signal {
        id: u32,
        state: Mutex<EventState>,
        ctx: egui::Context,
    }

    impl Signal {
        fn receive(&self, event: GlobalHotKeyEvent) {
            if event.id == self.id
                && self
                    .state
                    .lock()
                    .is_ok_and(|mut state| state.receive(event.state))
            {
                self.ctx.request_repaint();
            }
        }

        fn take_events(&self) -> Vec<ShortcutEvent> {
            self.state
                .lock()
                .map(|mut state| state.pending.drain(..).collect())
                .unwrap_or_default()
        }
    }

    struct Registration {
        manager: GlobalHotKeyManager,
        hotkey: HotKey,
        signal: Arc<Signal>,
        // macOS requires registration and destruction on its event-loop thread.
        _ui_thread: PhantomData<Rc<()>>,
    }

    impl Registration {
        fn register(ctx: egui::Context, code: Code, label: &str) -> Result<Self, String> {
            #[cfg(target_os = "linux")]
            {
                let wayland = std::env::var("XDG_SESSION_TYPE")
                    .is_ok_and(|session| session.eq_ignore_ascii_case("wayland"))
                    || std::env::var_os("WAYLAND_DISPLAY")
                        .is_some_and(|display| !display.is_empty());
                if wayland {
                    return Err("Global shortcuts are unavailable on Wayland. Use an X11 session or the in-app controls.".into());
                }
                if std::env::var_os("DISPLAY").is_none_or(|display| display.is_empty()) {
                    return Err("Global shortcuts require an X11 display on Linux.".into());
                }
            }
            let modifier = if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            };
            let hotkey = HotKey::new(Some(modifier | Modifiers::SHIFT), code);
            let manager = GlobalHotKeyManager::new()
                .map_err(|error| format!("Could not initialize the global shortcut: {error}"))?;
            let signal = Arc::new(Signal {
                id: hotkey.id(),
                state: Mutex::new(EventState::default()),
                ctx,
            });
            {
                let mut subscribers = SUBSCRIBERS
                    .lock()
                    .map_err(|_| "Global shortcut state is unavailable.".to_owned())?;
                subscribers.retain(|entry| entry.strong_count() != 0);
                if subscribers
                    .iter()
                    .filter_map(Weak::upgrade)
                    .any(|entry| entry.id == signal.id)
                {
                    return Err(format!(
                        "The global {label} shortcut is already registered."
                    ));
                }
                subscribers.push(Arc::downgrade(&signal));
            }
            INSTALL_HANDLER.call_once(|| {
                GlobalHotKeyEvent::set_event_handler(Some(|event| {
                    let signals = SUBSCRIBERS
                        .lock()
                        .map(|subscribers| {
                            subscribers
                                .iter()
                                .filter_map(Weak::upgrade)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    for signal in signals {
                        signal.receive(event);
                    }
                }));
            });
            let registration = Self {
                manager,
                hotkey,
                signal,
                _ui_thread: PhantomData,
            };
            registration.manager.register(hotkey).map_err(|error| format!("Could not register the global {label} shortcut. It may be in use by another app: {error}"))?;
            Ok(registration)
        }
    }

    impl Drop for Registration {
        fn drop(&mut self) {
            let _ = self.manager.unregister(self.hotkey);
            if let Ok(mut subscribers) = SUBSCRIBERS.lock() {
                let current = Arc::downgrade(&self.signal);
                subscribers.retain(|entry| !entry.ptr_eq(&current) && entry.strong_count() != 0);
            }
        }
    }

    pub struct SummonShortcut(Registration);
    impl SummonShortcut {
        pub fn register(ctx: egui::Context) -> Result<Self, String> {
            Registration::register(ctx, Code::Space, "summon").map(Self)
        }
        pub fn take_pressed(&self) -> bool {
            self.0
                .signal
                .take_events()
                .contains(&ShortcutEvent::Pressed)
        }
    }

    /// Separate opt-in voice chords; activation is for one utterance.
    /// Release events are available for capture implementations that support
    /// finishing, rather than cancelling, an active recording.
    pub struct VoiceShortcut(Registration);
    impl VoiceShortcut {
        pub fn register(ctx: egui::Context, kind: VoiceShortcutKind) -> Result<Self, String> {
            let (code, label) = match kind {
                VoiceShortcutKind::Talk => (Code::KeyV, "voice question"),
                VoiceShortcutKind::Dictate => (Code::KeyD, "dictation"),
            };
            Registration::register(ctx, code, label).map(Self)
        }
        pub fn take_events(&self) -> Vec<ShortcutEvent> {
            self.0.signal.take_events()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn signals_ignore_other_shortcuts_and_summon_rearms_on_release() {
            let signal = Signal {
                id: 42,
                state: Mutex::new(EventState::default()),
                ctx: egui::Context::default(),
            };
            let event = |id, state| GlobalHotKeyEvent { id, state };
            signal.receive(event(7, HotKeyState::Pressed));
            assert!(signal.take_events().is_empty());
            signal.receive(event(42, HotKeyState::Pressed));
            signal.receive(event(42, HotKeyState::Pressed));
            signal.receive(event(7, HotKeyState::Released));
            signal.receive(event(42, HotKeyState::Pressed));
            assert_eq!(signal.take_events(), vec![ShortcutEvent::Pressed]);
            signal.receive(event(42, HotKeyState::Released));
            signal.receive(event(42, HotKeyState::Pressed));
            assert_eq!(
                signal.take_events(),
                vec![ShortcutEvent::Released, ShortcutEvent::Pressed]
            );
        }
        #[test]
        fn transitions_are_ordered_and_repeated_presses_do_not_retrigger() {
            let mut state = EventState::default();
            assert!(!state.receive(HotKeyState::Released));
            assert!(state.receive(HotKeyState::Pressed));
            assert!(!state.receive(HotKeyState::Pressed));
            assert!(state.receive(HotKeyState::Released));
            assert!(!state.receive(HotKeyState::Released));
            assert!(state.receive(HotKeyState::Pressed));
            assert_eq!(
                state.pending.into_iter().collect::<Vec<_>>(),
                vec![
                    ShortcutEvent::Pressed,
                    ShortcutEvent::Released,
                    ShortcutEvent::Pressed
                ]
            );
        }
        #[test]
        fn stalled_gui_has_bounded_event_queue() {
            let mut state = EventState::default();
            for _ in 0..100 {
                state.receive(HotKeyState::Pressed);
                state.receive(HotKeyState::Released);
            }
            assert_eq!(state.pending.len(), MAX_PENDING_EVENTS);
            assert_eq!(state.pending.back(), Some(&ShortcutEvent::Released));
            assert!(!state.held);
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub struct SummonShortcut;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl SummonShortcut {
    pub fn register(_: eframe::egui::Context) -> Result<Self, String> {
        Err("Global shortcuts are unsupported on this platform.".into())
    }
    pub fn take_pressed(&self) -> bool {
        false
    }
}
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub struct VoiceShortcut;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl VoiceShortcut {
    pub fn register(_: eframe::egui::Context, _: VoiceShortcutKind) -> Result<Self, String> {
        Err("Global shortcuts are unsupported on this platform.".into())
    }
    pub fn take_events(&self) -> Vec<ShortcutEvent> {
        Vec::new()
    }
}
