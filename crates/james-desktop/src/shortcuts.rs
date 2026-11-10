//! Opt-in registration of one summon chord, scoped to the native UI thread.

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use native::SummonShortcut;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod native {
    use std::marker::PhantomData;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, Once, Weak};

    use eframe::egui;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

    // global-hotkey's handler is initialized once for the process. Keep that
    // dispatcher installed, but release its weak subscriber whenever disabled.
    static INSTALL_HANDLER: Once = Once::new();
    static SUBSCRIBER: Mutex<Option<Weak<Signal>>> = Mutex::new(None);

    struct Signal {
        id: u32,
        held: AtomicBool,
        pressed: AtomicBool,
        ctx: egui::Context,
    }

    impl Signal {
        fn receive(&self, event: GlobalHotKeyEvent) -> bool {
            if event.id != self.id {
                return false;
            }
            match event.state {
                HotKeyState::Pressed if !self.held.swap(true, Ordering::AcqRel) => {
                    self.pressed.store(true, Ordering::Release);
                    true
                }
                HotKeyState::Released => {
                    self.held.store(false, Ordering::Release);
                    false
                }
                _ => false,
            }
        }
    }

    pub struct SummonShortcut {
        manager: GlobalHotKeyManager,
        hotkey: HotKey,
        signal: Arc<Signal>,
        // Registration, unregister, and manager destruction stay on the event
        // loop thread even on platforms whose manager happens to be Send.
        _ui_thread: PhantomData<Rc<()>>,
    }

    impl SummonShortcut {
        /// Call from the running native GUI update/event-loop thread. macOS
        /// requires the main thread. Do not register from an app constructor or
        /// a worker thread; a running native event loop is required.
        pub fn register(ctx: egui::Context) -> Result<Self, String> {
            #[cfg(target_os = "linux")]
            {
                let wayland_session = std::env::var("XDG_SESSION_TYPE")
                    .is_ok_and(|session| session.eq_ignore_ascii_case("wayland"));
                let wayland_display =
                    std::env::var_os("WAYLAND_DISPLAY").is_some_and(|display| !display.is_empty());
                if wayland_session || wayland_display {
                    return Err("Global summon shortcut is unavailable on Wayland. Use the in-app shortcut or an X11 session.".into());
                }
                if std::env::var_os("DISPLAY").is_none_or(|display| display.is_empty()) {
                    return Err("Global summon shortcut requires an X11 display on Linux.".into());
                }
            }

            let modifier = if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            };
            let hotkey = HotKey::new(Some(modifier | Modifiers::SHIFT), Code::Space);
            let manager = GlobalHotKeyManager::new()
                .map_err(|error| format!("Could not initialize the global shortcut: {error}"))?;
            let signal = Arc::new(Signal {
                id: hotkey.id(),
                held: AtomicBool::new(false),
                pressed: AtomicBool::new(false),
                ctx,
            });
            {
                let mut subscriber = SUBSCRIBER
                    .lock()
                    .map_err(|_| "Global shortcut state is unavailable.".to_owned())?;
                if subscriber.as_ref().and_then(Weak::upgrade).is_some() {
                    return Err("The global summon shortcut is already registered.".into());
                }
                *subscriber = Some(Arc::downgrade(&signal));
            }
            INSTALL_HANDLER.call_once(|| {
                GlobalHotKeyEvent::set_event_handler(Some(|event| {
                    let signal = SUBSCRIBER
                        .lock()
                        .ok()
                        .and_then(|subscriber| subscriber.as_ref().and_then(Weak::upgrade));
                    if let Some(signal) = signal
                        && signal.receive(event)
                    {
                        // The GUI consumes the signal and changes its own state.
                        signal.ctx.request_repaint();
                    }
                }));
            });
            let shortcut = Self {
                manager,
                hotkey,
                signal,
                _ui_thread: PhantomData,
            };
            shortcut.manager.register(hotkey).map_err(|error| {
                format!("Could not register the global summon shortcut. It may be in use by another app: {error}")
            })?;
            Ok(shortcut)
        }

        /// Consume one pending press from the GUI thread without blocking.
        pub fn take_pressed(&self) -> bool {
            self.signal.pressed.swap(false, Ordering::AcqRel)
        }
    }

    impl Drop for SummonShortcut {
        fn drop(&mut self) {
            if let Ok(mut subscriber) = SUBSCRIBER.lock()
                && subscriber
                    .as_ref()
                    .is_some_and(|current| current.ptr_eq(&Arc::downgrade(&self.signal)))
            {
                *subscriber = None;
            }
            let _ = self.manager.unregister(self.hotkey);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn only_matching_press_signals_and_release_rearms() {
            let signal = Signal {
                id: 42,
                held: AtomicBool::new(false),
                pressed: AtomicBool::new(false),
                ctx: egui::Context::default(),
            };
            let event = |id, state| GlobalHotKeyEvent { id, state };
            assert!(!signal.receive(event(7, HotKeyState::Pressed)));
            assert!(!signal.pressed.load(Ordering::Acquire));
            assert!(signal.receive(event(42, HotKeyState::Pressed)));
            assert!(signal.pressed.swap(false, Ordering::AcqRel));
            assert!(!signal.receive(event(42, HotKeyState::Pressed)));
            assert!(!signal.receive(event(7, HotKeyState::Released)));
            assert!(!signal.receive(event(42, HotKeyState::Pressed)));
            assert!(!signal.receive(event(42, HotKeyState::Released)));
            assert!(!signal.pressed.load(Ordering::Acquire));
            assert!(signal.receive(event(42, HotKeyState::Pressed)));
            assert!(signal.pressed.load(Ordering::Acquire));
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub struct SummonShortcut;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl SummonShortcut {
    pub fn register(_: eframe::egui::Context) -> Result<Self, String> {
        Err("Global summon shortcuts are unsupported on this platform.".into())
    }

    pub fn take_pressed(&self) -> bool {
        false
    }
}
