use eframe::egui;
use eframe::egui::TextStyle;
use eframe::epaint::Vec2;
use std::error;
use std::fmt::Pointer;
use std::fs::read;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::time::timeout;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    join, process,
    sync::mpsc,
};

mod control;
mod mi;
mod parser;

use crate::control::{user_output, ControlState, InputCommand};

#[derive(Debug)]
pub enum ConsoleOutput {
    Stdout(String),
    Stderr(String),
}

struct MyApp {
    code: String,
    user_input: String,

    sender: mpsc::Sender<InputCommand>,
    reader_handle: tokio::task::JoinHandle<()>,

    console_output: Arc<Mutex<String>>,
    input_fields: Vec<String>,
    gdb_state: Arc<Mutex<control::ControlState>>,
}

impl MyApp {
    fn new(
        sender: mpsc::Sender<InputCommand>,
        mut receiver: mpsc::Receiver<ConsoleOutput>,
    ) -> MyApp {
        let console_handle = Arc::new(Mutex::new(String::new()));
        let reader_console_handle = console_handle.clone();

        let gdb_state_handle = Arc::new(Mutex::new(ControlState::new()));
        let reader_gdb_handle = gdb_state_handle.clone();

        let consume_console_handle = tokio::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                let mut console_out = reader_console_handle.lock().unwrap();

                let cur_state = {
                    let s = reader_gdb_handle.lock().unwrap().clone();
                    s
                };

                let next_state = control::read_console_input(cur_state, &cmd);

                {
                    *reader_gdb_handle.lock().unwrap() = next_state;
                }

                let cmd_str = match cmd {
                    ConsoleOutput::Stdout(s) => s,
                    ConsoleOutput::Stderr(s) => s,
                };

                match control::user_output(&cmd_str) {
                    Some(s) => console_out.push_str(&s),
                    _ => {}
                }

                //console_out.push_str(&cmd_str);
            }
        });

        let mut input_fields = vec![];
        input_fields.push("./res/a.out".to_string());
        for _ in 0..20 {
            input_fields.push("".to_string());
        }

        MyApp {
            code: include_str!("./main.rs").into(),
            user_input: String::new(),
            console_output: console_handle.clone(),
            sender,
            reader_handle: consume_console_handle,
            gdb_state: gdb_state_handle.clone(),
            input_fields,
        }
    }

    fn send_stdin(&self, input: &str) {
        let input_owned = input.to_string();
        let tx = self.sender.clone();
        tokio::spawn(async move {
            tx.send(InputCommand::StdinInput(input_owned))
                .await
                .unwrap();
        });
    }
}

impl eframe::epi::App for MyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &eframe::epi::Frame) {
        let cur_state = {
            let s = self.gdb_state.lock().unwrap().clone();
            s
        };

        let (next_state, cmds) = control::advance_cmds(&cur_state);

        {
            *self.gdb_state.lock().unwrap() = next_state.clone();
        }

        if &cur_state != &next_state {
            println!("New state");
        }

        let cur_state = next_state;

        for cmd in cmds {
            self.send_stdin(&cmd);
        }

        let mut buttons = vec![];

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::top("A")
                .resizable(true)
                .show_inside(ui, |ui| {
                    ui.heading("Iron Debugger");
                    ui.label("The debugger that's about to Rust");
                    ui.monospace(format!("state: {cur_state:?}"));

                    ui.horizontal(|ui| {
                        for btn in cur_state.buttons() {
                            buttons.push(ui.button(btn).clicked());
                        }
                    });

                    for (i, (label, input)) in cur_state.input_fields().iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.input_fields[i])
                                    .hint_text("...")
                                    .font(TextStyle::Monospace),
                            );
                        });
                    }
                });

            //egui::TopBottomPanel::bottom("Console")
            // .resizable(true)
            // .show_inside(ui, |ui| {
            egui::Window::new("Console").show(ctx, |ui| {
                let console = {
                    let data = self.console_output.lock().unwrap().clone();
                    data
                };

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        ui.monospace(&console);
                    });

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.user_input)
                            .hint_text("Write something")
                            .font(TextStyle::Monospace),
                    );

                    if ui.button("Send command").clicked() {
                        let tx = self.sender.clone();
                        let input = &self.user_input;

                        {
                            let mut data = self.console_output.lock().unwrap();
                            data.push_str(&format!("~> {}\n", input));
                        };

                        self.send_stdin(input);
                    }
                });
            });
        });

        if buttons.iter().any(|x| *x) {
            let next_state = control::read_button_input(cur_state, &buttons, &self.input_fields);
            *self.gdb_state.lock().unwrap() = next_state;
        }
    }

    fn name(&self) -> &str {
        "Iron Debugger"
    }
}

async fn console(mut rx: mpsc::Receiver<InputCommand>, mut tx: mpsc::Sender<ConsoleOutput>) {
    let mut cmd = process::Command::new("bash")
        //.arg("--interpreter=mi")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    let mut stdin = cmd.stdin.take().unwrap();
    let stdout = cmd.stdout.take().unwrap();
    let stderr = cmd.stderr.take().unwrap();

    let mut buf_stdout = BufReader::new(stdout);
    let mut buf_stderr = BufReader::new(stderr);

    let stderr_tx = tx.clone();

    // stdout
    let reader = tokio::spawn(async move {
        let mut s = String::new();

        while let Ok(_) = buf_stdout.read_line(&mut s).await {
            print!("{s}");
            tx.send(ConsoleOutput::Stdout(s.clone())).await.unwrap();
            s.clear();
        }
    });

    // stderr
    tokio::spawn(async move {
        let mut s = String::new();

        while let Ok(_) = buf_stderr.read_line(&mut s).await {
            print!("{s}");
            stderr_tx
                .send(ConsoleOutput::Stderr(s.clone()))
                .await
                .unwrap();
            s.clear();
        }
    });

    let writer = tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            use InputCommand::*;
            match cmd {
                StdinInput(s) => {
                    let mut s = s;
                    s.push('\n');
                    print!("~> {}", s);
                    if let Err(e) = stdin.write_all(s.as_bytes()).await {
                        println!("Failed to write to stdin, leaving");
                        break;
                    }
                    stdin.flush().await.unwrap();
                }
            }
        }
    });

    join!(writer);
    println!("Console done");
}

fn create_console_channels() -> (
    mpsc::Sender<InputCommand>,
    mpsc::Receiver<InputCommand>,
    mpsc::Sender<ConsoleOutput>,
    mpsc::Receiver<ConsoleOutput>,
) {
    let (tx_cmd, rx_cmd) = mpsc::channel(1);
    let (tx_out, rx_out) = mpsc::channel(1);

    (tx_cmd, rx_cmd, tx_out, rx_out)
}

async fn command_output(cmd: &str) -> String {
    let max_timeout = tokio::time::Duration::from_millis(2000);

    let (tx_cmd, rx_cmd, tx_out, mut rx_out) = create_console_channels();
    let console_handle = tokio::spawn(timeout(max_timeout, console(rx_cmd, tx_out)));
    println!("Spawned console");

    let exit = "exit\n";
    let out = Arc::new(Mutex::new(String::new()));
    let recv_out_handle = out.clone();

    let recv_handle = tokio::spawn(timeout(max_timeout, async move {
        if let Some(out) = rx_out.recv().await {
            let out_str = match out {
                ConsoleOutput::Stdout(s) => s,
                ConsoleOutput::Stderr(s) => s,
            };
            *recv_out_handle.lock().unwrap() = out_str;
        }
    }));

    tx_cmd
        .send(InputCommand::StdinInput(cmd.to_string()))
        .await
        .unwrap();

    println!("Sent the command");

    tx_cmd
        .send(InputCommand::StdinInput(exit.to_string()))
        .await
        .unwrap();

    println!("Sent all commands");

    //join!(console_handle);
    let first_output = join!(recv_handle).0.unwrap();

    let out_val = { out.lock().unwrap().clone() };
    out_val
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = (Some(Vec2::new(1000., 1000.)));

    let (tx_cmd, rx_cmd, tx_out, rx_out) = create_console_channels();

    let handle = tokio::spawn(console(rx_cmd, tx_out));

    eframe::run_native(Box::new(MyApp::new(tx_cmd, rx_out)), options);

    join!(handle);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_console() {
        let out = command_output("echo hello, there").await;
        assert_eq!(&out, "hello, there\n")
    }
}
