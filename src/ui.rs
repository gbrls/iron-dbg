use crate::control::ControlState;
use crate::egui::Color32;
use eframe::egui::{RichText, Ui};
use std::fs;

pub fn current_file(ui: &mut Ui, state: &ControlState) {
    match state {
        ControlState::GDBRunning {
            file: Some(p),
            line: Some(cur_line),
            ..
        } => {
            // We don't have the responsibility to handle an incorrect path here
            let contents = fs::read_to_string(p).unwrap();
            for (i, line) in contents.lines().enumerate() {
                let color = if i + 1 == (*cur_line) as usize {
                    Color32::from_rgb(255, 255, 0)
                } else {
                    Color32::from_rgb(100, 100, 100)
                };

                let line = format!("{:02} {}", i + 1, line);

                ui.monospace(RichText::new(line).color(color));
            }
        }
        _ => {}
    }
}

pub fn stack_frame(ui: &mut Ui, state: &ControlState) {
    match state {
        ControlState::GDBRunning {
            frames: Some(fs), ..
        } => {
            for f in fs {
                ui.monospace(format!("{f:?}"));
                ui.separator();
            }
        }
        _ => {}
    }
}
