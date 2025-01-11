use chatters::backend_actor::BackendActor;
use chatters::backends::{Backend, Error, Signal};
use chatters::commands::Command;
use chatters::keybinds::KeyBinds;
use chatters::message::{BackendMessage, FrontendMessage};
use chatters::tui::{render, Mode, TuiState};
use crossterm::event::Event;
use crossterm::event::EventStream;
use crossterm::event::KeyEvent;
use directories::ProjectDirs;
use futures::channel::mpsc;
use futures::future::Either;
use futures::pin_mut;
use futures::{future::select, StreamExt};
use presage::libsignal_service::prelude::Uuid;
use presage::store::Thread;
use qrcode_generator::QrCodeEcc;
use ratatui::DefaultTerminal;
use tui_input::backend::crossterm::EventHandler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let project_dirs = ProjectDirs::from("net", "jeffas", "chatters-signal").unwrap();
    let db_path = project_dirs.data_local_dir();

    let device_name = "chatters-test".to_owned();

    let backend = match Signal::load(&db_path).await {
        Ok(b) => b,
        Err(Error::Unlinked) => {
            let (provisioning_link_tx, provisioning_link_rx) = futures::channel::oneshot::channel();
            let backend = futures::future::join(
                Signal::link(&db_path, &device_name, provisioning_link_tx),
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
                        Err(error) => eprintln!("linking device was cancelled: {}", error),
                    }
                },
            )
            .await;
            backend.0.unwrap()
        }
    };

    let self_uuid = backend.self_uuid().await;

    eprintln!("loaded signal backend");

    let mut backend2 = backend.clone();

    // eprintln!("syncing contacts");
    // backend.sync_contacts().await.unwrap();
    // eprintln!("synced contacts");

    let (b_tx, b_rx) = mpsc::unbounded();
    let (f_tx, f_rx) = mpsc::unbounded();

    let mut ba = BackendActor {
        backend,
        message_rx: b_rx,
        message_tx: f_tx.clone(),
    };

    let ui = async move {
        let terminal = ratatui::init();
        run_ui(terminal, b_tx, f_rx, self_uuid).await;
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
        backend2.background_sync(f_tx).await.unwrap();
    };
    pin_mut!(sync);

    let backend = async move {
        select(actor, sync).await;
    };
    pin_mut!(backend);

    select(frontend, backend).await;

    Ok(())
}

async fn run_ui(
    mut terminal: DefaultTerminal,
    backend_actor_tx: mpsc::UnboundedSender<BackendMessage>,
    mut backend_actor_rx: mpsc::UnboundedReceiver<FrontendMessage>,
    self_uuid: Uuid,
) {
    // select on two channels, one for keyboard events, another for messages from the backend
    // (responses)
    //
    // handle either action then render the ui again

    let mut tui_state = TuiState::default();
    tui_state.self_uuid = self_uuid;

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
                if process_user_event(&mut tui_state, &backend_actor_tx, event) {
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
    event: Event,
) -> bool {
    // dbg!(&event);
    let normal_keybinds = KeyBinds::normal_default();
    let command_keybinds = KeyBinds::command_default();
    let compose_keybinds = KeyBinds::compose_default();

    let mode = tui_state.mode;

    let mut execute_command = |cmd: &Box<dyn Command>| {
        match cmd.execute(tui_state, ba_tx) {
            Ok(cf) => {
                if cf.is_break() {
                    return true;
                }
            }
            Err(error) => {
                tui_state.command_error = error.to_string();
            }
        }
        false
    };

    match &event {
        Event::Key(KeyEvent { code, .. }) => match mode {
            Mode::Normal => {
                if let Some(command) = normal_keybinds.get(&code) {
                    if execute_command(command) {
                        return true;
                    }
                }
            }
            Mode::Command => {
                if let Some(command) = command_keybinds.get(&code) {
                    if execute_command(command) {
                        return true;
                    }
                } else {
                    tui_state.command.handle_event(&event);
                }
            }
            Mode::Compose => {
                if let Some(command) = compose_keybinds.get(&code) {
                    if execute_command(command) {
                        return true;
                    }
                } else {
                    tui_state.compose.handle_event(&event);
                }
            }
        },
        e => {
            eprintln!("unhandled event {e:?}");
        }
    }
    false
}

fn process_backend_message(
    tui_state: &mut TuiState,
    ba_tx: &mpsc::UnboundedSender<BackendMessage>,
    msg: FrontendMessage,
) {
    // dbg!(&msg);
    match msg {
        FrontendMessage::LoadedContacts(vec) => {
            if tui_state.contacts.is_empty() && !vec.is_empty() {
                tui_state.contact_list_state.select_next();
            }
            tui_state.contacts = vec;
            tui_state.contacts_by_id = tui_state
                .contacts
                .iter()
                .filter_map(|c| {
                    if let Thread::Contact(uuid) = c.thread_id {
                        Some((uuid, c.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            if let Some(contact) = tui_state
                .contact_list_state
                .selected()
                .and_then(|i| tui_state.contacts.get(i))
            {
                ba_tx
                    .unbounded_send(BackendMessage::LoadMessages(contact.thread_id.clone()))
                    .unwrap();
            }
        }
        FrontendMessage::LoadedMessages(vec) => {
            if tui_state.messages.is_empty() && !vec.is_empty() {
                tui_state.message_list_state.select_last();
            }
            tui_state.messages.clear();
            tui_state.messages.extend(vec);
        }
        FrontendMessage::NewMessage(m) => {
            if let Some((i, contact)) = tui_state
                .contact_list_state
                .selected()
                .and_then(|i| tui_state.contacts.get_mut(i).map(|c| (i, c)))
            {
                if m.thread == contact.thread_id {
                    contact.last_message_timestamp = m.timestamp;
                    let c = tui_state.contacts.remove(i);
                    tui_state.contacts.insert(0, c);
                    tui_state.contact_list_state.select(Some(0));

                    tui_state.messages.add_single(m);
                }
            }
        }
        FrontendMessage::Tick => {
            // do nothing, just trigger a UI redraw
        }
    }
}
