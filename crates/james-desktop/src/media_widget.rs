//! Explicitly requested now-playing information and controls.
use crate::media_now::{self, MediaAction, NowPlaying};
use eframe::egui;
use james_platform::Cancellation;
use std::sync::mpsc::{self, Receiver, TryRecvError};

enum MediaResult {
    Query(Result<Option<NowPlaying>, String>),
    Control(Result<(), String>),
}

#[derive(Default)]
pub struct MediaWidget {
    snapshot: Option<NowPlaying>,
    pending: Option<Receiver<MediaResult>>,
    cancel: Option<Cancellation>,
    message: Option<String>,
}

impl MediaWidget {
    fn start(&mut self, ctx: &egui::Context, action: Option<MediaAction>) {
        if self.pending.is_some() {
            return;
        }
        let snapshot = self.snapshot.clone();
        let cancel = Cancellation::default();
        let worker_cancel = cancel.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = if let Some(action) = action {
                MediaResult::Control(
                    snapshot
                        .as_ref()
                        .ok_or_else(|| "Refresh media first.".to_owned())
                        .and_then(|snapshot| media_now::control(snapshot, action, &worker_cancel)),
                )
            } else {
                MediaResult::Query(media_now::query(&worker_cancel))
            };
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.pending = Some(rx);
        self.cancel = Some(cancel);
        self.message = None;
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.pending.take() else {
            return;
        };
        match rx.try_recv() {
            Err(TryRecvError::Empty) => self.pending = Some(rx),
            Err(TryRecvError::Disconnected) => {
                self.cancel = None;
                self.message = Some("Media helper stopped unexpectedly.".into());
            }
            Ok(MediaResult::Query(Ok(snapshot))) => {
                self.cancel = None;
                self.snapshot = snapshot;
                self.message = self
                    .snapshot
                    .is_none()
                    .then(|| "No current media session.".into());
            }
            Ok(MediaResult::Control(Ok(()))) => {
                self.cancel = None;
                self.snapshot = None;
                self.message =
                    Some("Player accepted the request. Refresh to see its current state.".into());
                ctx.request_repaint();
            }
            Ok(MediaResult::Query(Err(error)) | MediaResult::Control(Err(error))) => {
                self.cancel = None;
                self.message = Some(error);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll(ui.ctx());
        ui.strong("Media");
        if !cfg!(windows) {
            ui.small("Media controls are currently available on Windows only.");
            return;
        }
        if let Some(snapshot) = &self.snapshot {
            ui.label(format!("{} — {}", snapshot.title, snapshot.artist));
            ui.small(format!(
                "{} · {}",
                snapshot.source,
                if snapshot.playing {
                    "Playing"
                } else {
                    "Not playing"
                }
            ));
        }
        let ready = self.pending.is_none();
        let toggle = self.snapshot.as_ref().is_some_and(|s| s.can_toggle);
        let next = self.snapshot.as_ref().is_some_and(|s| s.can_next);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(ready, egui::Button::new("Refresh media"))
                .clicked()
            {
                self.start(ui.ctx(), None);
            }
            if ui
                .add_enabled(ready && toggle, egui::Button::new("Play / pause"))
                .clicked()
            {
                self.start(ui.ctx(), Some(MediaAction::Toggle));
            }
            if ui
                .add_enabled(ready && next, egui::Button::new("Next track"))
                .clicked()
            {
                self.start(ui.ctx(), Some(MediaAction::Next));
            }
        });
        if self.pending.is_some() {
            ui.small("Checking media…");
            if ui.button("Stop media request").clicked()
                && let Some(cancel) = &self.cancel
            {
                cancel.cancel();
            }
        }
        if let Some(message) = &self.message {
            ui.small(message);
        }
        if self.snapshot.is_some() {
            ui.small("If the same app has several media sessions, Windows may select another session from that app.");
        }
    }
}

impl Drop for MediaWidget {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
    }
}
