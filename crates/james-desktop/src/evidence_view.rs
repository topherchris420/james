//! Read-only UI over the latest in-memory request record.
use crate::{JamesApp, evidence};
use eframe::egui::{self, RichText};

pub fn answer(app: &mut JamesApp, ui: &mut egui::Ui, index: usize) {
    let Some(receipt) = &app.receipt else {
        return;
    };
    if receipt.question_index + 1 != index || receipt.sources.is_empty() {
        return;
    }
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Check the sources").strong());
            if ui.small_button("Inspect request and sources").clicked() {
                app.show_evidence = true;
            }
        });
        let references = evidence::citations(
            receipt.answer.as_deref().unwrap_or_default(),
            &receipt.sources,
        );
        if references.is_empty() {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "No source references found in the inspected text.",
            );
        } else {
            let unresolved = references.iter().filter(|r| r.source.is_none()).count();
            egui::CollapsingHeader::new(format!(
                "{} located · {unresolved} unresolved",
                references.len() - unresolved
            ))
            .id_salt(("references", index))
            .default_open(references.len() <= 3)
            .show(ui, |ui| {
                ui.small("Check whether the cited lines support the answer.");
                for (n, reference) in references.iter().enumerate() {
                    if let Some(source) = reference.source {
                        let source = &receipt.sources[source];
                        egui::CollapsingHeader::new(format!(
                            "{} · {}",
                            reference.reference, source.name
                        ))
                        .id_salt(("citation", index, n))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(180.0)
                                .show(ui, |ui| {
                                    for line in reference.start..=reference.end {
                                        ui.monospace(format!(
                                            "L{line} | {}",
                                            source.lines[line - 1]
                                        ));
                                    }
                                });
                        });
                    } else {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            format!("{} · Source or line range unavailable", reference.reference),
                        );
                    }
                }
            });
        }
    });
}

pub fn window(app: &mut JamesApp, ctx: &egui::Context) {
    if !app.show_evidence {
        return;
    }
    let mut open = true;
    let mut forget = false;
    egui::Window::new("Request evidence")
        .open(&mut open)
        .default_width(600.0)
        .show(ctx, |ui| {
            let Some(receipt) = &app.receipt else {
                ui.label("Send a chat question to inspect its request here.");
                return;
            };
            ui.label(format!("{} · {} · 127.0.0.1:{}", receipt.model, receipt.provider, receipt.port));
            let timing = receipt.elapsed_ms.map(|ms| format!(" · {:.1} s", ms as f64 / 1000.0)).unwrap_or_default();
            ui.label(format!("Request: {}{timing}", receipt.state));
            if let Some(note) = &receipt.outcome_note { ui.label(note); }
            ui.small(format!("{} text messages · {} text bytes · {} document(s) · {} older messages omitted", receipt.context.len(), receipt.context.iter().map(|m| m.content.len()).sum::<usize>(), receipt.sources.len(), receipt.omitted_messages));
            ui.small(format!("Image: {}", receipt.image));
            ui.small("Kept in memory for the latest request. The local model server may relay requests elsewhere.");
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for source in &receipt.sources {
                    egui::CollapsingHeader::new(format!("{} · {} · {} lines", source.id, source.name, source.lines.len()))
                        .id_salt(("source", &source.id))
                        .show(ui, |ui| { ui.monospace(source.numbered()); });
                }
                egui::CollapsingHeader::new("Exact prepared text messages")
                    .show(ui, |ui| {
                        ui.small("Text prepared before provider encoding. Image pixels are excluded.");
                        for (i, message) in receipt.context.iter().enumerate() {
                            ui.push_id(i, |ui| {
                                ui.label(RichText::new(format!("{} · {}", i + 1, message.role)).strong());
                                // Render source data literally, never as clickable Markdown.
                                ui.monospace(&message.content);
                                ui.separator();
                            });
                        }
                    });
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copy evidence JSON").on_hover_text("Copies document text, prompts and the answer to your clipboard.").clicked() {
                    ctx.copy_text(receipt.export());
                }
                forget = ui.add_enabled(!app.busy(), egui::Button::new("Forget request evidence")).clicked();
            });
            ui.small("Copy includes document text and prompts. Review before sharing.");
        });
    app.show_evidence = open && !forget;
    if forget {
        app.receipt = None;
    }
}
