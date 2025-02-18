use std::{collections::BTreeMap, process::Stdio};

use log::warn;

use crate::backends::{Contact, Message};

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hooks {
    pub on_new_message: Option<String>,
}

impl Hooks {
    pub fn do_on_new_message(&self, contact: &Contact, sender: &Contact, message: &Message) {
        let Some(script) = &self.on_new_message else {
            return;
        };

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(
            "CHATTERS_APP_NAME".to_owned(),
            std::env::args().next().unwrap(),
        );
        envs.insert("CHATTERS_CONTACT_NAME".to_owned(), contact.name.clone());
        envs.insert("CHATTERS_SENDER_NAME".to_owned(), sender.name.clone());
        envs.insert(
            "CHATTERS_MESSAGE_BODY".to_owned(),
            message.content.to_string(),
        );

        let child = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .envs(envs)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if let Err(error) = child {
            warn!(error:?; "Failed to execute on_new_message hook");
        }
    }
}
