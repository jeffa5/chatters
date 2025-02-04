use crate::backends::Error;

use super::{Backend, ContactId};
use log::debug;
use matrix_sdk::matrix_auth::MatrixSession;
use matrix_sdk::LoopCtrl;
use matrix_sdk::{config::SyncSettings, Client};
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
    async fn load(path: &std::path::Path) -> super::Result<Self> {
        let session_file = get_session_file(path);
        if !session_file.exists() {
            return Err(Error::Unlinked);
        }
        // The session was serialized as JSON in a file.
        let serialized_session = std::fs::read_to_string(session_file).unwrap();
        let FullSession {
            client_session,
            user_session,
            sync_token,
        } = serde_json::from_str(&serialized_session).unwrap();

        // Build the client with the previous settings from the session.
        let client = Client::builder()
            .homeserver_url(client_session.homeserver)
            .sqlite_store(client_session.db_path, Some(&client_session.passphrase))
            .build()
            .await
            .unwrap();

        println!("Restoring session for {}…", user_session.meta.user_id);

        // Restore the Matrix user session.
        client.restore_session(user_session).await.unwrap();

        Ok(Self { client })
    }

    async fn link(
        path: &std::path::Path,
        device_name: &str,
        provisioning_link_tx: futures::channel::oneshot::Sender<url::Url>,
    ) -> super::Result<Self> {
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

        // After logging in, you might want to verify this session with another one (see
        // the `emoji_verification` example), or bootstrap cross-signing if this is your
        // first session with encryption, or if you need to reset cross-signing because
        // you don't have access to your old sessions (see the
        // `cross_signing_bootstrap` example).

        Ok(Self { client })
    }

    async fn sync_contacts(&mut self) -> super::Result<()> {
        todo!()
    }

    async fn background_sync(
        &mut self,
        ba_tx: futures::channel::mpsc::UnboundedSender<crate::message::FrontendMessage>,
    ) -> super::Result<()> {
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

    async fn contacts(&self) -> super::Result<Vec<super::Contact>> {
        todo!()
    }

    async fn groups(&self) -> super::Result<Vec<super::Contact>> {
        todo!()
    }

    async fn messages(
        &mut self,
        contact: ContactId,
        start_ts: std::ops::Bound<u64>,
        end_ts: std::ops::Bound<u64>,
    ) -> super::Result<Vec<super::Message>> {
        todo!()
    }

    async fn send_message(
        &mut self,
        contact: ContactId,
        body: super::MessageContent,
        quoting: Option<&super::Quote>,
    ) -> super::Result<super::Message> {
        todo!()
    }

    async fn self_id(&self) -> Vec<u8> {
        self.client.user_id().unwrap().as_bytes().to_vec()
    }

    async fn download_attachment(&self, attachment_index: usize) -> super::Result<String> {
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
            .sqlite_store(&db_path, Some(&passphrase))
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
