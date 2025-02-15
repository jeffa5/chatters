use chatters_lib::backends::timestamp;
use chatters_lib::backends::Backend;
use chatters_lib::backends::Contact;
use chatters_lib::backends::ContactId;
use chatters_lib::backends::Error;
use chatters_lib::backends::Message;
use chatters_lib::backends::MessageContent;
use chatters_lib::backends::Quote;
use chatters_lib::backends::Result;
use chatters_lib::message::FrontendMessage;

use futures::future::select;
use futures::{pin_mut, StreamExt as _};
use log::debug;
use matrix_sdk::crypto::{format_emojis, Emoji, SasState};
use matrix_sdk::encryption::verification::{
    SasVerification, Verification, VerificationRequest, VerificationRequestState,
};
use matrix_sdk::matrix_auth::MatrixSession;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::RoomId;
use matrix_sdk::{config::SyncSettings, Client};
use matrix_sdk::{LoopCtrl, RoomMemberships};
use rand::distr::Alphanumeric;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use std::io::{self, Write as _};
use std::path::Path;
use std::path::PathBuf;

/// The data needed to re-build a client.
#[derive(Debug, Serialize, Deserialize)]
struct ClientSession {
    /// The URL of the homeserver of the user.
    homeserver: String,

    /// The path of the database.
    db_path: PathBuf,

    /// The passphrase of the database.
    passphrase: String,
}

/// The full session to persist.
#[derive(Debug, Serialize, Deserialize)]
struct FullSession {
    /// The data to re-build the client.
    client_session: ClientSession,

    /// The Matrix user session.
    user_session: MatrixSession,

    /// The latest sync token.
    ///
    /// It is only needed to persist it when using `Client::sync_once()` and we
    /// want to make our syncs faster by not receiving all the initial sync
    /// again.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync_token: Option<String>,
}

#[derive(Clone)]
pub struct Matrix {
    client: Client,
}

impl Backend for Matrix {
    async fn load(path: &std::path::Path) -> Result<Self> {
        let session_file = get_session_file(path);
        if !session_file.exists() {
            return Err(Error::Unlinked);
        }
        // The session was serialized as JSON in a file.
        let serialized_session = std::fs::read_to_string(session_file).unwrap();
        let FullSession {
            client_session,
            user_session,
            sync_token: _,
        } = serde_json::from_str(&serialized_session).unwrap();

        // Build the client with the previous settings from the session.
        let client = Client::builder()
            .homeserver_url(client_session.homeserver)
            .sqlite_store(client_session.db_path, Some(&client_session.passphrase))
            .build()
            .await
            .unwrap();

        let user_id = user_session.meta.user_id.clone();
        println!("Restoring session for {}…", user_id);

        // Restore the Matrix user session.
        client.restore_session(user_session).await.unwrap();

        if !client.logged_in() {
            debug!("Client not logged in after load, getting to link again");
            return Err(Error::Unlinked);
        }

        let this_user = client
            .encryption()
            .get_user_identity(&user_id)
            .await
            .unwrap()
            .unwrap();
        debug!(user:? = this_user, verified:? = this_user.is_verified(); "Loading");
        if !this_user.is_verified() {
            verify(&client).await;
        }

        Ok(Self { client })
    }

    async fn link(
        path: &std::path::Path,
        _device_name: &str,
        _provisioning_link_tx: futures::channel::oneshot::Sender<url::Url>,
    ) -> Result<Self> {
        let (client, client_session) = build_client(path).await.unwrap();
        let matrix_auth = client.matrix_auth();

        loop {
            print!("\nUsername: ");
            io::stdout().flush().expect("Unable to write to stdout");
            let mut username = String::new();
            io::stdin()
                .read_line(&mut username)
                .expect("Unable to read user input");
            username = username.trim().to_owned();

            print!("Password: ");
            io::stdout().flush().expect("Unable to write to stdout");
            let mut password = String::new();
            io::stdin()
                .read_line(&mut password)
                .expect("Unable to read user input");
            password = password.trim().to_owned();

            match matrix_auth
                .login_username(&username, &password)
                .initial_device_display_name("chatters-matrix")
                .await
            {
                Ok(_) => {
                    println!("Logged in as {username}");
                    break;
                }
                Err(error) => {
                    println!("Error logging in: {error}");
                    println!("Please try again\n");
                }
            }
        }

        // Persist the session to reuse it later.
        // This is not very secure, for simplicity. If the system provides a way of
        // storing secrets securely, it should be used instead.
        // Note that we could also build the user session from the login response.
        let user_session = matrix_auth
            .session()
            .expect("A logged-in client should have a session");
        let serialized_session = serde_json::to_string(&FullSession {
            client_session,
            user_session,
            sync_token: None,
        })
        .unwrap();

        let session_file = get_session_file(path);
        std::fs::write(&session_file, serialized_session).unwrap();

        debug!(
            "Matrix session persisted in {}",
            session_file.to_string_lossy()
        );

        verify(&client).await;

        Ok(Self { client })
    }

    async fn background_sync(
        &mut self,
        _ba_tx: futures::channel::mpsc::UnboundedSender<FrontendMessage>,
    ) -> Result<()> {
        let sync_settings = SyncSettings::default();
        self.client
            .sync_with_result_callback(sync_settings, |sync_result| async move {
                let response = sync_result?;
                debug!(response:?; "Got sync response");

                // We persist the token each time to be able to restore our session
                // persist_sync_token(session_file, response.next_batch)
                //     .await
                //     .map_err(|err| Error::UnknownError(err.into()))?;

                Ok(LoopCtrl::Continue)
            })
            .await
            .unwrap();
        Ok(())
    }

    async fn users(&self) -> Result<Vec<Contact>> {
        let rooms = self.client.rooms();
        for room in rooms {
            debug!(room:?; "Found room");
        }
        let rooms = self.client.joined_rooms();
        let mut users = Vec::new();
        for room in rooms {
            let member_count = room.members(RoomMemberships::JOIN).await.unwrap().len();
            debug!(member_count:?; "Found room");
            if member_count > 2 {
                continue;
            }

            let user = Contact {
                id: ContactId::User(room.room_id().as_bytes().to_vec()),
                name: room
                    .compute_display_name()
                    .await
                    .map_or(room.room_id().to_string(), |n| n.to_string()),
                address: String::new(),
                last_message_timestamp: 0,
                description: String::new(),
            };
            users.push(user);
        }
        Ok(users)
    }

    async fn groups(&self) -> Result<Vec<Contact>> {
        let rooms = self.client.joined_rooms();
        let mut groups = Vec::new();
        for room in rooms {
            let member_count = room.members(RoomMemberships::JOIN).await.unwrap().len();
            debug!(member_count:?; "Found room");
            if member_count <= 2 {
                continue;
            }

            let group = Contact {
                id: ContactId::Group(room.room_id().as_bytes().to_vec()),
                name: room.name().unwrap(),
                address: String::new(),
                last_message_timestamp: 0,
                description: String::new(),
            };
            groups.push(group);
        }
        Ok(groups)
    }

    async fn messages(
        &mut self,
        contact: ContactId,
        _start_ts: std::ops::Bound<u64>,
        _end_ts: std::ops::Bound<u64>,
    ) -> Result<Vec<Message>> {
        let contact_bytes = match contact {
            ContactId::User(vec) => vec,
            ContactId::Group(vec) => vec,
        };
        let contact_str = String::from_utf8(contact_bytes).unwrap();
        let room_id = RoomId::parse(contact_str).unwrap();

        let messages = Vec::new();

        if let Some(room) = self.client.get_room(&room_id) {
            let messages = room.messages(MessagesOptions::forward()).await.unwrap();
            debug!(start:? = messages.start, end:? = messages.end; "Got some messages");
            for event in messages.chunk {
                debug!(event:? = event; "Got timeline event");
            }
        }

        Ok(messages)
    }

    async fn send_message(
        &mut self,
        contact: ContactId,
        content: MessageContent,
        quoting: Option<&Quote>,
    ) -> Result<Message> {
        let contact_bytes = match &contact {
            ContactId::User(vec) => vec,
            ContactId::Group(vec) => vec,
        }
        .clone();
        let contact_str = String::from_utf8(contact_bytes).unwrap();
        let room_id = RoomId::parse(contact_str).unwrap();

        let room = self.client.get_room(&room_id).unwrap();
        let matrix_content = match &content {
            MessageContent::Text {
                text,
                attachments: _,
            } => {
                
                RoomMessageEventContent::text_plain(text)
            }
            MessageContent::Reaction {
                message_author: _,
                timestamp: _,
                reaction: _,
                remove: _,
            } => todo!(),
        };

        room.send(matrix_content).await.unwrap();

        let quote = quoting.map(|quoted| Quote {
            timestamp: quoted.timestamp,
            sender: quoted.sender.clone(),
            text: quoted.text.clone(),
        });
        Ok(Message {
            timestamp: timestamp(),
            sender: self.self_id().await,
            contact_id: contact,
            content,
            quote,
        })
    }

    async fn self_id(&self) -> Vec<u8> {
        self.client.user_id().unwrap().as_bytes().to_vec()
    }

    async fn download_attachment(&self, _attachment_index: usize) -> Result<PathBuf> {
        todo!()
    }
}

fn get_session_file(path: &Path) -> PathBuf {
    path.parent().unwrap().join("session.json")
}

/// Build a new client.
async fn build_client(data_dir: &Path) -> anyhow::Result<(Client, ClientSession)> {
    let mut rng = rand::rng();

    let db_path = data_dir;

    // Generate a random passphrase.
    let passphrase: String = (&mut rng)
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // We create a loop here so the user can retry if an error happens.
    loop {
        let mut homeserver = String::new();

        print!("Homeserver URL: ");
        io::stdout().flush().expect("Unable to write to stdout");
        io::stdin()
            .read_line(&mut homeserver)
            .expect("Unable to read user input");

        println!("\nChecking homeserver…");

        match Client::builder()
            .homeserver_url(&homeserver)
            .sqlite_store(db_path, Some(&passphrase))
            .build()
            .await
        {
            Ok(client) => {
                return Ok((
                    client,
                    ClientSession {
                        homeserver,
                        db_path: db_path.to_owned(),
                        passphrase,
                    },
                ))
            }
            Err(error) => match &error {
                matrix_sdk::ClientBuildError::AutoDiscovery(_)
                | matrix_sdk::ClientBuildError::Url(_)
                | matrix_sdk::ClientBuildError::Http(_) => {
                    println!("Error checking the homeserver: {error}");
                    println!("Please try again\n");
                }
                _ => {
                    // Forward other errors, it's unlikely we can retry with a different outcome.
                    return Err(error.into());
                }
            },
        }
    }
}

async fn verify(client: &Client) {
    println!("Verifying device, please accept the request on another of your device");
    let this_user = client
        .encryption()
        .request_user_identity(client.user_id().unwrap())
        .await
        .unwrap()
        .unwrap();
    let verification_request = this_user.request_verification().await.unwrap();

    // TODO: cancel this after verification
    let c = client.clone();
    let sync = c.sync(SyncSettings::new());
    pin_mut!(sync);

    let verify = request_verification_handler(verification_request);
    pin_mut!(verify);

    // cancels the other future on drop
    select(sync, verify).await;
}

async fn request_verification_handler(request: VerificationRequest) {
    debug!(
        other_user_id:? = request.other_user_id();
        "Accepting verification request",
    );
    request
        .accept()
        .await
        .expect("Can't accept verification request");
    debug!("Accepted our side during verification");

    let mut stream = request.changes();

    while let Some(state) = stream.next().await {
        debug!(state:?; "Got state during verification");
        match state {
            VerificationRequestState::Created { .. }
            | VerificationRequestState::Requested { .. }
            | VerificationRequestState::Ready { .. } => (),
            VerificationRequestState::Transitioned { verification } => {
                // We only support SAS verification.
                if let Verification::SasV1(s) = verification {
                    sas_verification_handler(s).await;
                    break;
                }
            }
            VerificationRequestState::Done | VerificationRequestState::Cancelled(_) => break,
        }
    }
}

async fn wait_for_confirmation(emoji: [Emoji; 7]) -> bool {
    loop {
        println!("\nDo the emojis match: \n{}", format_emojis(emoji.clone()));
        print!("Confirm with `yes` or cancel with `no`: ");
        std::io::stdout()
            .flush()
            .expect("We should be able to flush stdout");

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("error: unable to read user input");

        match input.trim().to_lowercase().as_ref() {
            "yes" | "true" | "ok" => return true,
            "no" | "false" => return false,
            _ => {}
        }
        println!(
            "\nSorry, {} isn't one of the expected values, please answer yes or no.",
            input.trim()
        );
    }
}

async fn sas_verification_handler(sas: SasVerification) {
    println!(
        "Starting verification with {} {}",
        &sas.other_device().user_id(),
        &sas.other_device().device_id()
    );
    sas.accept().await.unwrap();

    let mut stream = sas.changes();

    while let Some(state) = stream.next().await {
        match state {
            SasState::KeysExchanged {
                emojis,
                decimals: _,
            } => {
                let success = wait_for_confirmation(
                    emojis
                        .expect("We only support verifications using emojis")
                        .emojis,
                )
                .await;
                if success {
                    sas.confirm().await.unwrap();
                } else {
                    sas.cancel().await.unwrap();
                }
            }
            SasState::Done { .. } => {
                let device = sas.other_device();

                println!(
                    "Successfully verified device {} {} {:?}",
                    device.user_id(),
                    device.device_id(),
                    device.local_trust_state()
                );

                break;
            }
            SasState::Cancelled(cancel_info) => {
                println!(
                    "The verification has been cancelled, reason: {}",
                    cancel_info.reason()
                );

                break;
            }
            SasState::Created { .. }
            | SasState::Started { .. }
            | SasState::Accepted { .. }
            | SasState::Confirmed => (),
        }
    }
}
