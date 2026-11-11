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
        ui.label(RichText::new("Check the sources").strong());
        ui.small("These documents were sent with this question only. Reattach them for a source-based follow-up.");
        ui.small("Checks up to 64 source-reference attempts; repeated references are shown once.");
        let references = evidence::citations(receipt.answer.as_deref().unwrap_or_default(), &receipt.sources);
        if references.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, "No source references were recognized in the inspected text. Check the documents before relying on this answer.");
        } else {
            let unresolved = references.iter().filter(|r| r.source.is_none()).count();
            ui.small(format!("{} reference(s) locate supplied lines; {unresolved} unresolved. This does not verify the claims.", references.len() - unresolved));
            for (n, reference) in references.iter().enumerate() {
                if let Some(source) = reference.source {
                    let source = &receipt.sources[source];
                    egui::CollapsingHeader::new(format!("{} · {}", reference.reference, source.name))
                        .id_salt(("citation", index, n))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                                for line in reference.start..=reference.end {
                                    ui.monospace(format!("L{line} | {}", source.lines[line - 1]));
                                }
                            });
                        });
                } else {
                    ui.colored_label(ui.visuals().warn_fg_color, format!("{} — source or line range unavailable", reference.reference));
                }
            }
        }
        if ui.button("Inspect request and sources").clicked() {
            app.show_evidence = true;
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
            ui.heading("What this answer was based on");
            ui.label(format!("{} · {} · 127.0.0.1:{}", receipt.model, receipt.provider, receipt.port));
            let timing = receipt.elapsed_ms.map(|ms| format!(" · {:.1} s", ms as f64 / 1000.0)).unwrap_or_default();
            ui.label(format!("Request: {}{timing}", receipt.state));
            if let Some(note) = &receipt.outcome_note { ui.label(note); }
            ui.small(format!("{} text messages · {} text bytes · {} document(s) · {} older messages omitted", receipt.context.len(), receipt.context.iter().map(|m| m.content.len()).sum::<usize>(), receipt.sources.len(), receipt.omitted_messages));
            ui.small(format!("Image: {}", receipt.image));
            ui.small("This records the prepared request, not proof the server processed it. Image pixels are not retained here. Your local model server may relay requests elsewhere.");
            ui.separator();
            ui.label("Kept only in memory until the next chat, guidance or action-proposal request, a conversation change, Forget, or exit. It is never saved with history.");
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copy evidence JSON").on_hover_text("Copies source text, prepared prompts and the answer to your clipboard. Review before sharing.").clicked() {
                    ctx.copy_text(receipt.export());
                }
                forget = ui.add_enabled(!app.busy(), egui::Button::new("Forget request evidence")).clicked();
            });
            ui.small("Copy includes document contents and prompts. Review before sharing.");
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for source in &receipt.sources {
                    egui::CollapsingHeader::new(format!("{} · {} · {} lines", source.id, source.name, source.lines.len()))
                        .id_salt(("source", &source.id))
                        .show(ui, |ui| { ui.monospace(source.numbered()); });
                }
                egui::CollapsingHeader::new("Exact prepared text messages")
                    .show(ui, |ui| {
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
        });
    app.show_evidence = open && !forget;
    if forget {
        app.receipt = None;
    }
}
