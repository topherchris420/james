//! Opt-in global shortcuts, registered and released on the native UI thread.

use serde::{Deserialize, Serialize};

/// A finite set of physical keys keeps persisted chords small and reviewable.
/// Letter shortcuts use the key position, independently of the keyboard layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShortcutKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Space,
}

impl ShortcutKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Digit0 => "0",
            Self::Digit1 => "1",
            Self::Digit2 => "2",
            Self::Digit3 => "3",
            Self::Digit4 => "4",
            Self::Digit5 => "5",
            Self::Digit6 => "6",
            Self::Digit7 => "7",
            Self::Digit8 => "8",
            Self::Digit9 => "9",
            Self::Space => "Space",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        const KEYS: [ShortcutKey; 37] = [
            ShortcutKey::A,
            ShortcutKey::B,
            ShortcutKey::C,
            ShortcutKey::D,
            ShortcutKey::E,
            ShortcutKey::F,
            ShortcutKey::G,
            ShortcutKey::H,
            ShortcutKey::I,
            ShortcutKey::J,
            ShortcutKey::K,
            ShortcutKey::L,
            ShortcutKey::M,
            ShortcutKey::N,
            ShortcutKey::O,
            ShortcutKey::P,
            ShortcutKey::Q,
            ShortcutKey::R,
            ShortcutKey::S,
            ShortcutKey::T,
            ShortcutKey::U,
            ShortcutKey::V,
            ShortcutKey::W,
            ShortcutKey::X,
            ShortcutKey::Y,
            ShortcutKey::Z,
            ShortcutKey::Digit0,
            ShortcutKey::Digit1,
            ShortcutKey::Digit2,
            ShortcutKey::Digit3,
            ShortcutKey::Digit4,
            ShortcutKey::Digit5,
            ShortcutKey::Digit6,
            ShortcutKey::Digit7,
            ShortcutKey::Digit8,
            ShortcutKey::Digit9,
            ShortcutKey::Space,
        ];
        KEYS.into_iter()
            .find(|key| key.label().eq_ignore_ascii_case(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortcutChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: ShortcutKey,
}

impl ShortcutChord {
    fn default_for(key: ShortcutKey) -> Self {
        Self {
            ctrl: !cfg!(target_os = "macos"),
            alt: false,
            shift: true,
            meta: cfg!(target_os = "macos"),
            key,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        if !self.ctrl && !self.alt && !self.meta {
            return Err("Include Ctrl, Alt, or Meta/Cmd so normal typing stays available.".into());
        }
        Ok(())
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        if value.len() > 64 || !value.is_ascii() {
            return Err("Use a shortcut of at most 64 ASCII characters.".into());
        }
        let mut chord = Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            key: ShortcutKey::Space,
        };
        let mut key = None;
        for token in value.split('+').map(str::trim) {
            let modifier = match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Some(&mut chord.ctrl),
                "alt" | "option" => Some(&mut chord.alt),
                "shift" => Some(&mut chord.shift),
                "meta" | "cmd" | "command" | "win" | "super" => Some(&mut chord.meta),
                _ => None,
            };
            if let Some(modifier) = modifier {
                if *modifier {
                    return Err("Each modifier may appear only once.".into());
                }
                *modifier = true;
            } else if let Some(parsed) = ShortcutKey::parse(token) {
                if key.replace(parsed).is_some() {
                    return Err("Choose exactly one letter, digit, or Space key.".into());
                }
            } else {
                return Err("Use Ctrl, Alt, Shift, Meta/Cmd and one letter, digit, or Space, separated by +.".into());
            }
        }
        chord.key = key.ok_or("Choose a letter, digit, or Space key after the modifiers.")?;
        chord.validate()?;
        Ok(chord)
    }

    pub fn label(self) -> String {
        let mut parts = Vec::with_capacity(5);
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.meta {
            parts.push(if cfg!(target_os = "macos") {
                "Cmd"
            } else {
                "Meta"
            });
        }
        parts.push(self.key.label());
        parts.join("+")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShortcutConfig {
    pub summon: ShortcutChord,
    pub talk: ShortcutChord,
    pub dictate: ShortcutChord,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            summon: ShortcutChord::default_for(ShortcutKey::Space),
            talk: ShortcutChord::default_for(ShortcutKey::V),
            dictate: ShortcutChord::default_for(ShortcutKey::D),
        }
    }
}

impl ShortcutConfig {
    pub fn validate_enabled(self, summon: bool, talk: bool, dictate: bool) -> Result<(), String> {
        let entries = [
            (summon, self.summon, "Summon"),
            (talk, self.talk, "Talk"),
            (dictate, self.dictate, "Dictate"),
        ];
        for (index, (enabled, chord, label)) in entries.iter().enumerate() {
            // Validate disabled choices too, so enabling one cannot activate an
            // unmodified key loaded from a manually edited preferences file.
            chord
                .validate()
                .map_err(|error| format!("{label}: {error}"))?;
            if *enabled {
                for (other_enabled, other_chord, other_label) in &entries[..index] {
                    if *other_enabled && chord == other_chord {
                        return Err(format!(
                            "{label} and {other_label} cannot use the same enabled shortcut."
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

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

    use super::{ShortcutChord, ShortcutEvent, ShortcutKey, VoiceShortcutKind};
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
        chord: ShortcutChord,
        registered: bool,
        // macOS requires registration and destruction on its event-loop thread.
        _ui_thread: PhantomData<Rc<()>>,
    }

    impl Registration {
        fn register(ctx: egui::Context, chord: ShortcutChord, label: &str) -> Result<Self, String> {
            chord.validate()?;
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
            let mut modifiers = Modifiers::empty();
            if chord.ctrl {
                modifiers |= Modifiers::CONTROL;
            }
            if chord.alt {
                modifiers |= Modifiers::ALT;
            }
            if chord.shift {
                modifiers |= Modifiers::SHIFT;
            }
            if chord.meta {
                modifiers |= Modifiers::META;
            }
            let code = match chord.key {
                ShortcutKey::A => Code::KeyA,
                ShortcutKey::B => Code::KeyB,
                ShortcutKey::C => Code::KeyC,
                ShortcutKey::D => Code::KeyD,
                ShortcutKey::E => Code::KeyE,
                ShortcutKey::F => Code::KeyF,
                ShortcutKey::G => Code::KeyG,
                ShortcutKey::H => Code::KeyH,
                ShortcutKey::I => Code::KeyI,
                ShortcutKey::J => Code::KeyJ,
                ShortcutKey::K => Code::KeyK,
                ShortcutKey::L => Code::KeyL,
                ShortcutKey::M => Code::KeyM,
                ShortcutKey::N => Code::KeyN,
                ShortcutKey::O => Code::KeyO,
                ShortcutKey::P => Code::KeyP,
                ShortcutKey::Q => Code::KeyQ,
                ShortcutKey::R => Code::KeyR,
                ShortcutKey::S => Code::KeyS,
                ShortcutKey::T => Code::KeyT,
                ShortcutKey::U => Code::KeyU,
                ShortcutKey::V => Code::KeyV,
                ShortcutKey::W => Code::KeyW,
                ShortcutKey::X => Code::KeyX,
                ShortcutKey::Y => Code::KeyY,
                ShortcutKey::Z => Code::KeyZ,
                ShortcutKey::Digit0 => Code::Digit0,
                ShortcutKey::Digit1 => Code::Digit1,
                ShortcutKey::Digit2 => Code::Digit2,
                ShortcutKey::Digit3 => Code::Digit3,
                ShortcutKey::Digit4 => Code::Digit4,
                ShortcutKey::Digit5 => Code::Digit5,
                ShortcutKey::Digit6 => Code::Digit6,
                ShortcutKey::Digit7 => Code::Digit7,
                ShortcutKey::Digit8 => Code::Digit8,
                ShortcutKey::Digit9 => Code::Digit9,
                ShortcutKey::Space => Code::Space,
            };
            let hotkey = HotKey::new(Some(modifiers), code);
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
            let mut registration = Self {
                manager,
                hotkey,
                signal,
                chord,
                registered: false,
                _ui_thread: PhantomData,
            };
            registration.manager.register(hotkey).map_err(|error| format!("Could not register the global {label} shortcut. It may be in use by another app: {error}"))?;
            registration.registered = true;
            Ok(registration)
        }
    }

    impl Drop for Registration {
        fn drop(&mut self) {
            if self.registered {
                let _ = self.manager.unregister(self.hotkey);
            }
            if let Ok(mut subscribers) = SUBSCRIBERS.lock() {
                let current = Arc::downgrade(&self.signal);
                subscribers.retain(|entry| !entry.ptr_eq(&current) && entry.strong_count() != 0);
            }
        }
    }

    fn replace_registration<T>(
        current: &mut T,
        register: impl FnOnce() -> Result<T, String>,
    ) -> Result<(), String> {
        let replacement = register()?;
        *current = replacement;
        Ok(())
    }

    pub struct SummonShortcut(Registration);
    impl SummonShortcut {
        pub fn register_chord(ctx: egui::Context, chord: ShortcutChord) -> Result<Self, String> {
            Registration::register(ctx, chord, "summon").map(Self)
        }
        pub fn chord(&self) -> ShortcutChord {
            self.0.chord
        }
        /// Register the replacement before releasing the old binding, so a
        /// rejected chord leaves the existing shortcut working.
        pub fn update_chord(
            &mut self,
            ctx: egui::Context,
            chord: ShortcutChord,
        ) -> Result<(), String> {
            if self.chord() != chord {
                replace_registration(self, || Self::register_chord(ctx, chord))?;
            }
            Ok(())
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
        pub fn register_chord(
            ctx: egui::Context,
            kind: VoiceShortcutKind,
            chord: ShortcutChord,
        ) -> Result<Self, String> {
            let label = match kind {
                VoiceShortcutKind::Talk => "voice question",
                VoiceShortcutKind::Dictate => "dictation",
            };
            Registration::register(ctx, chord, label).map(Self)
        }
        pub fn chord(&self) -> ShortcutChord {
            self.0.chord
        }
        pub fn update_chord(
            &mut self,
            ctx: egui::Context,
            kind: VoiceShortcutKind,
            chord: ShortcutChord,
        ) -> Result<(), String> {
            if self.chord() != chord {
                replace_registration(self, || Self::register_chord(ctx, kind, chord))?;
            }
            Ok(())
        }
        pub fn take_events(&self) -> Vec<ShortcutEvent> {
            self.0.signal.take_events()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn rejected_rebind_keeps_old_registration_and_success_releases_it_after_registration() {
            use std::cell::RefCell;
            struct Resource(&'static str, Rc<RefCell<Vec<&'static str>>>);
            impl Drop for Resource {
                fn drop(&mut self) {
                    self.1.borrow_mut().push(self.0);
                }
            }
            let released = Rc::new(RefCell::new(Vec::new()));
            let mut registration = Resource("old", released.clone());
            let failure = replace_registration(&mut registration, || Err("in use".into()));
            assert_eq!(failure.unwrap_err(), "in use");
            assert_eq!(registration.0, "old");
            assert!(released.borrow().is_empty());
            replace_registration(&mut registration, || {
                assert!(
                    released.borrow().is_empty(),
                    "old binding released before registering replacement"
                );
                Ok(Resource("new", released.clone()))
            })
            .unwrap();
            assert_eq!(registration.0, "new");
            assert_eq!(&*released.borrow(), &["old"]);
            drop(registration);
            assert_eq!(&*released.borrow(), &["old", "new"]);
        }
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
    pub fn register_chord(_: eframe::egui::Context, _: ShortcutChord) -> Result<Self, String> {
        Err("Global shortcuts are unsupported on this platform.".into())
    }
    pub fn chord(&self) -> ShortcutChord {
        ShortcutConfig::default().summon
    }
    pub fn update_chord(
        &mut self,
        _: eframe::egui::Context,
        _: ShortcutChord,
    ) -> Result<(), String> {
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
    pub fn register_chord(
        _: eframe::egui::Context,
        _: VoiceShortcutKind,
        _: ShortcutChord,
    ) -> Result<Self, String> {
        Err("Global shortcuts are unsupported on this platform.".into())
    }
    pub fn chord(&self) -> ShortcutChord {
        ShortcutConfig::default().talk
    }
    pub fn update_chord(
        &mut self,
        _: eframe::egui::Context,
        _: VoiceShortcutKind,
        _: ShortcutChord,
    ) -> Result<(), String> {
        Err("Global shortcuts are unsupported on this platform.".into())
    }
    pub fn take_events(&self) -> Vec<ShortcutEvent> {
        Vec::new()
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn configurable_chords_are_normalized_and_serializable() {
        let chord = ShortcutChord::parse(" alt + SHIFT + control + 7 ").unwrap();
        assert_eq!(chord.label(), "Ctrl+Alt+Shift+7");
        assert_eq!(ShortcutChord::parse(&chord.label()).unwrap(), chord);
        assert_eq!(
            ShortcutChord::parse("Command+space").unwrap(),
            ShortcutChord::parse("meta+SPACE").unwrap()
        );
        let config = ShortcutConfig {
            summon: chord,
            ..ShortcutConfig::default()
        };
        let serialized = serde_json::to_string(&config).unwrap();
        assert_eq!(
            serde_json::from_str::<ShortcutConfig>(&serialized).unwrap(),
            config
        );
        assert!(serialized.len() < 350);
    }

    #[test]
    fn chords_reject_ambiguous_or_unmodified_typing_keys() {
        for invalid in [
            "",
            "Ctrl+Shift",
            "A",
            "Shift+A",
            "Ctrl+A+B",
            "Ctrl+Ctrl+A",
            "Ctrl+Control+A",
            "Ctrl++A",
            "Ctrl+F1",
            "Ctrl+é",
            "Ctrl+Enter",
            "Meta+Cmd+A",
        ] {
            assert!(ShortcutChord::parse(invalid).is_err(), "accepted {invalid}");
        }
        assert!(ShortcutChord::parse(&format!("Ctrl+{}A", " ".repeat(64))).is_err());
        for key in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars() {
            let chord = ShortcutChord::parse(&format!("Ctrl+{key}")).unwrap();
            assert_eq!(chord.key.label(), key.to_string());
        }
    }

    #[test]
    fn default_chords_are_distinct_and_old_empty_settings_use_defaults() {
        let config = ShortcutConfig::default();
        config.validate_enabled(true, true, true).unwrap();
        assert_eq!(config.summon.key, ShortcutKey::Space);
        assert_eq!(config.talk.key, ShortcutKey::V);
        assert_eq!(config.dictate.key, ShortcutKey::D);
        assert_eq!(
            serde_json::from_str::<ShortcutConfig>("{}").unwrap(),
            config
        );
    }

    #[test]
    fn duplicate_enabled_chords_and_invalid_stored_modifiers_are_rejected() {
        let mut config = ShortcutConfig::default();
        config.dictate = config.talk;
        assert!(
            config
                .validate_enabled(false, true, true)
                .unwrap_err()
                .contains("Dictate and Talk")
        );
        config.validate_enabled(false, true, false).unwrap();
        config.dictate.ctrl = false;
        config.dictate.meta = false;
        assert!(config.validate_enabled(false, false, false).is_err());
        assert!(serde_json::from_str::<ShortcutConfig>(r#"{"extra":true}"#).is_err());
        assert!(serde_json::from_str::<ShortcutKey>(r#""Enter""#).is_err());
    }
}
