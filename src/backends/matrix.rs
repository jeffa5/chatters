use super::Backend;
use log::debug;
use matrix_sdk::matrix_auth::MatrixSession;
use matrix_sdk::{config::SyncSettings, ruma::events::room::message::SyncRoomMessageEvent, Client};
use serde::Deserialize;
use serde::Serialize;
use std::io::{self, Write as _};
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
        Err(super::Error::Unlinked)
    }

    async fn link(
        path: &std::path::Path,
        device_name: &str,
        provisioning_link_tx: futures::channel::oneshot::Sender<url::Url>,
    ) -> super::Result<Self> {
        let session_file = path.parent().unwrap().join("session.json");

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
                .initial_device_display_name("persist-session client")
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

        let passphrase = "3pR8h4D4JaG@nKFi@DL$";

        let client = Client::builder()
            .server_name(user.server_name())
            .sqlite_store(path, Some(&passphrase))
            .build()
            .await
            .unwrap();

        let client_session = ClientSession {
            homeserver: client.homeserver().to_string(),
            db_path: path.to_path_buf(),
            passphrase: passphrase.to_owned(),
        };

        debug!(user:?; "Logging in");

        // First we need to log in.
        client
            .matrix_auth()
            .login_username(user, &passphrase)
            .initial_device_display_name("chatters-matrix")
            .await
            .unwrap();

        debug!("Logged in");

        let user_session = client
            .matrix_auth()
            .session()
            .expect("A logged-in client should have a session");
        let serialized_session = serde_json::to_string(&FullSession {
            client_session,
            user_session,
            sync_token: None,
        })
        .unwrap();
        std::fs::write(session_file, serialized_session).unwrap();

        client.add_event_handler(|ev: SyncRoomMessageEvent| async move {
            println!("Received a message {:?}", ev);
        });

        // Syncing is important to synchronize the client state with the server.
        // This method will never return unless there is an error.
        client.sync(SyncSettings::default()).await.unwrap();

        Ok(Self { client })
    }

    async fn sync_contacts(&mut self) -> super::Result<()> {
        todo!()
    }

    async fn background_sync(
        &mut self,
        ba_tx: futures::channel::mpsc::UnboundedSender<crate::message::FrontendMessage>,
    ) -> super::Result<()> {
        todo!()
    }

    async fn contacts(&self) -> super::Result<Vec<super::Contact>> {
        todo!()
    }

    async fn groups(&self) -> super::Result<Vec<super::Contact>> {
        todo!()
    }

    async fn messages(
        &mut self,
        contact: presage::store::Thread,
        start_ts: std::ops::Bound<u64>,
        end_ts: std::ops::Bound<u64>,
    ) -> super::Result<Vec<super::Message>> {
        todo!()
    }

    async fn send_message(
        &mut self,
        contact: presage::store::Thread,
        body: super::MessageContent,
        quoting: Option<&super::Quote>,
    ) -> super::Result<super::Message> {
        todo!()
    }

    async fn self_uuid(&self) -> uuid::Uuid {
        todo!()
    }

    async fn download_attachment(&self, attachment_index: usize) -> super::Result<String> {
        todo!()
    }
}

/// Build a new client.
async fn build_client(data_dir: &Path) -> anyhow::Result<(Client, ClientSession)> {
    let mut rng = rand::thread_rng();

    let db_path = data_dir.join("db");

    // Generate a random passphrase.
    let passphrase: String = (&mut rng)
        .sample_iter(rand::distributions::Alphanumeric)
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
            // We use the SQLite store, which is enabled by default. This is the crucial part to
            // persist the encryption setup.
            // Note that other store backends are available and you can even implement your own.
            .sqlite_store(&db_path, Some(&passphrase))
            .build()
            .await
        {
            Ok(client) => {
                return Ok((
                    client,
                    ClientSession {
                        homeserver,
                        db_path,
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
