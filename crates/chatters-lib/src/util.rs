use crate::commands::{
    self, Command as _, CommandMode, ExecuteCommand, NextCommand, NormalMode, PrevCommand,
};
use crate::config::Config;
use crate::keybinds::KeyEvents;
use crate::message::BackendMessage;
use crate::tui::{render, Mode, TuiState};
use crate::{
    backend_actor::BackendActor,
    backends::{Backend, Error},
    message::FrontendMessage,
};
use crossterm::event::{Event, EventStream};
use crossterm::event::{KeyCode, KeyEvent};
use futures::channel::mpsc;
use futures::future::Either;
use futures::StreamExt as _;
use futures::{future::select, pin_mut};
use log::{debug, info, warn};
use qrcode_generator::QrCodeEcc;
use ratatui::prelude::CrosstermBackend;
use ratatui::{DefaultTerminal, Terminal};
use std::ffi::OsString;
use std::io::Stdout;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr as _;

#[derive(Debug, Clone)]
pub struct Options {
    pub device_name: String,
    pub data_local_dir: PathBuf,
    pub config_file: PathBuf,
}

pub async fn run<B: Backend + Clone>(options: Options) {
    let db_path = options.data_local_dir.join("db");

    let config = load_config(&options.config_file);
    debug!(config:?; "Loaded config file");

    let backend = match B::load(&db_path).await {
        Ok(b) => b,
        Err(Error::Unlinked) => {
            let (provisioning_link_tx, provisioning_link_rx) = futures::channel::oneshot::channel();
            let backend = futures::future::join(
                B::link(&db_path, &options.device_name, provisioning_link_tx),
                async move {
                    match provisioning_link_rx.await {
                        Ok(url) => {
                            let qr_path = "/tmp/signal_link_device_qrcode.svg";
                            println!(
                                "Please scan in the QR code from the image file at {qr_path:?}"
                            );
                            let qr_svg = qrcode_generator::to_svg_to_string(
                                url.to_string(),
                                QrCodeEcc::Low,
                                1024,
                                None::<String>,
                            )
                            .unwrap();
                            std::fs::write(qr_path, qr_svg.as_bytes()).unwrap();
                            open::that(qr_path).unwrap();
                        }
                        Err(error) => warn!(error:% = error; "Linking device was cancelled"),
                    }
                },
            )
            .await;
            backend.0.unwrap()
        }
        Err(_) => {
            unimplemented!()
        }
    };

    let self_id = backend.self_id().await;

    info!("Loaded backend");

    let mut backend2 = backend.clone();

    let (b_tx, b_rx) = mpsc::unbounded();
    let (f_tx, f_rx) = mpsc::unbounded();

    let mut ba = BackendActor {
        backend,
        message_rx: b_rx,
        message_tx: f_tx.clone(),
    };

    let ui = async move {
        let terminal = ratatui::init();
        run_ui(terminal, b_tx, f_rx, self_id, &config, options.config_file).await;
        ratatui::restore();
    };
    pin_mut!(ui);

    let f_tx2 = f_tx.clone();
    let tick = async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            f_tx2.unbounded_send(FrontendMessage::Tick).unwrap();
        }
    };
    pin_mut!(tick);

    let frontend = async move {
        select(ui, tick).await;
    };
    pin_mut!(frontend);

    let actor = async move { ba.run().await };
    pin_mut!(actor);

    let sync = async move {
        info!("Starting background sync");
        backend2.background_sync(f_tx).await.unwrap();
    };
    pin_mut!(sync);

    let backend = async move {
        select(actor, sync).await;
    };
    pin_mut!(backend);

    select(frontend, backend).await;
}

async fn run_ui(
    mut terminal: DefaultTerminal,
    backend_actor_tx: mpsc::UnboundedSender<BackendMessage>,
    mut backend_actor_rx: mpsc::UnboundedReceiver<FrontendMessage>,
    self_id: Vec<u8>,
    config: &Config,
    config_path: PathBuf,
) {
    // select on two channels, one for keyboard events, another for messages from the backend
    // (responses)
    //
    // handle either action then render the ui again

    let mut tui_state = TuiState::default();
    tui_state.self_id = self_id;
    tui_state.config = config.clone();
    tui_state.config_path = config_path;

    let mut event_stream = EventStream::new();

    backend_actor_tx
        .unbounded_send(BackendMessage::LoadContacts)
        .unwrap();

    loop {
        // dbg!(&tui_state);
        terminal.draw(|f| render(f, &mut tui_state)).unwrap();

        let event_future = async { event_stream.next().await.unwrap().unwrap() };
        pin_mut!(event_future);

        let backend_future = async { backend_actor_rx.next().await.unwrap() };
        pin_mut!(backend_future);

        match select(event_future, backend_future).await {
            Either::Left((event, _)) => {
                if process_user_event(
                    &mut tui_state,
                    &backend_actor_tx,
                    &mut terminal,
                    config,
                    event,
                ) {
                    break;
                }
            }
            Either::Right((message, _)) => {
                process_backend_message(&mut tui_state, &backend_actor_tx, message);
            }
        }
    }
}

fn process_user_event(
    tui_state: &mut TuiState,
    ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    config: &Config,
    event: Event,
) -> bool {
    // dbg!(&event);

    let mode = tui_state.mode;

    debug!(event:? = event; "Received event");
    match event {
        Event::Key(KeyEvent {
            code, modifiers, ..
        }) => {
            // handle builtin keybindings
            if code == KeyCode::Char(':')
                && modifiers.is_empty()
                && tui_state.key_events.0.is_empty()
                && !matches!(tui_state.mode, Mode::Compose)
            {
                if let Err(error) = CommandMode.execute(tui_state, ba_tx) {
                    tui_state.command_line.error = error.to_string();
                }
                return false;
            }
            if code == KeyCode::Esc && modifiers.is_empty() && tui_state.key_events.0.is_empty() {
                if let Err(error) = NormalMode.execute(tui_state, ba_tx) {
                    tui_state.command_line.error = error.to_string();
                }
                return false;
            }

            let key_event = crate::keybinds::KeyEvent { code, modifiers };
            tui_state.key_events.0.push(key_event);
            debug!(key_events:? = tui_state.key_events; "Looking for a key binding");
            match mode {
                Mode::Normal => {
                    match config.keybinds.get(&tui_state.key_events, mode) {
                        Ok(command) => {
                            if execute_command(tui_state, ba_tx, terminal, config, command.clone())
                            {
                                return true;
                            }
                        }
                        Err(true) => {
                            // could still be a binding with more input, leave it
                        }
                        Err(false) => {
                            tui_state.command_line.error =
                                format!("Failed to find keybind for {}", tui_state.key_events);
                            // no binding starts with this
                            tui_state.key_events.0.clear();
                        }
                    }
                }
                Mode::Command { previous: _ } => {
                    // currently don't support keybinds in command mode as simulation uses command
                    // mode itself
                    tui_state.key_events.0.clear();
                    if code == KeyCode::Tab {
                        // complete existing command
                        let cmd = tui_state.command_line.text();
                        if cmd.contains(' ') {
                            let args = shell_words::split(cmd)
                                .unwrap()
                                .into_iter()
                                .map(OsString::from)
                                .collect();
                            let mut pargs = pico_args::Arguments::from_vec(args);
                            let subcmd = pargs.subcommand().unwrap().unwrap();
                            let cmds = commands::commands();
                            let Some(mut command) = cmds
                                .into_iter()
                                .find(|c| c.names().contains(&subcmd.as_str()))
                            else {
                                return false;
                            };
                            let _ = command.parse(pargs);
                            let completions = command.complete(tui_state);
                            tui_state.command_line.set_completions(completions);
                        } else {
                            let commands = commands::commands();
                            let completions = commands
                                .into_iter()
                                .flat_map(|c| c.names().into_iter().filter(|n| n.starts_with(cmd)))
                                .map(|s| s.to_owned())
                                .collect::<Vec<_>>();
                            if completions.len() == 1 {
                                tui_state.command_line.set_text(completions[0].clone());
                            } else if completions.len() > 1 {
                                tui_state.command_line.set_completions(completions);
                            }
                        }
                    } else if code == KeyCode::Enter {
                        match ExecuteCommand.execute(tui_state, ba_tx) {
                            Ok(cs) => match cs {
                                commands::CommandSuccess::Nothing => {}
                                commands::CommandSuccess::Quit => return true,
                                commands::CommandSuccess::Clear => {
                                    terminal.clear().unwrap();
                                }
                            },
                            Err(error) => {
                                tui_state.command_line.error = error.to_string();
                            }
                        }
                    } else if code == KeyCode::Up {
                        if let Err(error) = PrevCommand.execute(tui_state, ba_tx) {
                            tui_state.command_line.error = error.to_string();
                        }
                    } else if code == KeyCode::Down {
                        if let Err(error) = NextCommand.execute(tui_state, ba_tx) {
                            tui_state.command_line.error = error.to_string();
                        }
                    } else {
                        tui_state.command_line.input(crossterm::event::KeyEvent {
                            code,
                            modifiers,
                            kind: crossterm::event::KeyEventKind::Press,
                            state: crossterm::event::KeyEventState::empty(),
                        });
                    }
                }
                Mode::Compose => {
                    match config.keybinds.get(&tui_state.key_events, mode) {
                        Ok(command) => {
                            if execute_command(tui_state, ba_tx, terminal, config, command.clone())
                            {
                                return true;
                            }
                        }
                        Err(true) => {
                            // skip
                        }
                        Err(false) => {
                            for key_event in tui_state.key_events.0.drain(..) {
                                tui_state.compose.input(crossterm::event::KeyEvent {
                                    code: key_event.code,
                                    modifiers: key_event.modifiers,
                                    kind: crossterm::event::KeyEventKind::Press,
                                    state: crossterm::event::KeyEventState::empty(),
                                });
                            }
                        }
                    }
                }
                Mode::Popup => match config.keybinds.get(&tui_state.key_events, mode) {
                    Ok(command) => {
                        if execute_command(tui_state, ba_tx, terminal, config, command.clone()) {
                            return true;
                        }
                    }
                    Err(true) => {
                        // prefix
                    }
                    Err(false) => {
                        tui_state.command_line.error =
                            format!("Failed to find keybind for {}", tui_state.key_events);
                        tui_state.key_events.0.clear();
                    }
                },
            }
        }
        e => {
            warn!(event:? = e; "Unhandled event");
        }
    }
    false
}

fn execute_command(
    tui_state: &mut TuiState,
    ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    config: &Config,
    cmd: String,
) -> bool {
    tui_state.key_events.0.clear();
    // break cmd into keyevents, then loop through them calling process_user_event for each
    debug!(cmd:?; "Executing command from keybinding");
    let key_events = KeyEvents::from_str(&cmd).unwrap();
    debug!(key_events:?; "Broke keybinding command up into key_events");
    let mut quit = false;
    for key_event in key_events.0 {
        debug!(key_event:?; "Simulating key event");
        quit = process_user_event(
            tui_state,
            ba_tx,
            terminal,
            config,
            Event::Key(KeyEvent {
                code: key_event.code,
                modifiers: key_event.modifiers,
                kind: crossterm::event::KeyEventKind::Press,
                state: crossterm::event::KeyEventState::empty(),
            }),
        );
    }
    quit
}

fn process_backend_message(
    tui_state: &mut TuiState,
    ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    msg: FrontendMessage,
) {
    // dbg!(&msg);
    match msg {
        FrontendMessage::LoadedContacts { contacts } => {
            if tui_state.contacts.is_empty() && !contacts.is_empty() {
                tui_state.contacts.state.select_next();
            }
            tui_state.contacts.clear();
            tui_state.contacts.extend(contacts);
            if let Some(contact) = tui_state.contacts.selected() {
                ba_tx
                    .unbounded_send(BackendMessage::LoadMessages {
                        contact_id: contact.id.clone(),
                        start_ts: std::ops::Bound::Unbounded,
                        end_ts: std::ops::Bound::Unbounded,
                    })
                    .unwrap();
            }
        }
        FrontendMessage::LoadedMessages { messages } => {
            if let Some(contact) = tui_state.contacts.selected_mut() {
                let last_message = messages.last().unwrap();
                if last_message.contact_id == contact.id {
                    contact.last_message_timestamp = Some(last_message.timestamp);
                    if tui_state.messages.is_empty() && !messages.is_empty() {
                        tui_state.messages.state.select_last();
                    }
                    tui_state.messages.clear();
                    tui_state.messages.extend(messages);
                }
            }
        }
        FrontendMessage::NewMessage { message } => {
            if let Some((i, contact)) = tui_state.contacts.state.selected().and_then(|i| {
                tui_state
                    .contacts
                    .contact_or_group_by_index_mut(i)
                    .map(|c| (i, c))
            }) {
                contact.last_message_timestamp = Some(message.timestamp);
                if message.contact_id == contact.id {
                    tui_state.contacts.move_by_index(i, 0);
                    tui_state.contacts.state.select(Some(0));

                    tui_state.messages.add_single(message);
                }
            }
        }
        FrontendMessage::DownloadedAttachment {
            contact_id,
            timestamp,
            index,
            file_path: file_name,
        } => {
            if let Some(contact) = tui_state
                .contacts
                .state
                .selected()
                .and_then(|i| tui_state.contacts.contact_or_group_by_index_mut(i))
            {
                if contact_id == contact.id {
                    if let Some(msg) = tui_state.messages.get_mut_by_timestamp(timestamp) {
                        // mark attachment as downloaded
                        let attachment = msg
                            .attachments
                            .iter_mut()
                            .find(|a| a.index == index)
                            .unwrap();
                        attachment.path = Some(file_name);
                    }
                }
            }
        }
        FrontendMessage::Tick => {
            // do nothing, just trigger a UI redraw
        }
    }
}

pub fn load_config(path: &Path) -> Config {
    let content = std::fs::read_to_string(path).expect("Config file was missing");
    toml::from_str(&content).expect("Malformed config file")
}
