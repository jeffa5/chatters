use std::collections::BTreeMap;

use ratatui::widgets::TableState;

use crate::backends::{Contact, ContactId};

#[derive(Debug, Default)]
pub struct Contacts {
    contacts_and_groups: Vec<Contact>,
    contacts_by_id: BTreeMap<Vec<u8>, Contact>,
    pub state: TableState,
}

impl Contacts {
    pub fn new(contacts_and_groups: Vec<Contact>) -> Self {
        let contacts_by_id = contacts_and_groups
            .iter()
            .filter_map(|c| {
                if let ContactId::User(id) = &c.id {
                    Some((id.clone(), c.clone()))
                } else {
                    None
                }
            })
            .collect();
        Self {
            contacts_and_groups,
            contacts_by_id,
            state: TableState::default(),
        }
    }

    pub fn contact_or_group_by_index(&self, index: usize) -> Option<&Contact> {
        self.contacts_and_groups.get(index)
    }

    pub fn contact_or_group_by_index_mut(&mut self, index: usize) -> Option<&mut Contact> {
        self.contacts_and_groups.get_mut(index)
    }

    pub fn contact_by_id(&self, id: &Vec<u8>) -> Option<&Contact> {
        self.contacts_by_id.get(id)
    }

    pub fn iter_contacts_and_groups(&self) -> impl Iterator<Item = &Contact> {
        self.contacts_and_groups.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.contacts_and_groups.is_empty()
    }

    pub fn len(&self) -> usize {
        self.contacts_and_groups.len()
    }

    pub fn clear(&mut self) {
        self.contacts_and_groups.clear();
        self.contacts_by_id.clear();
    }

    pub fn move_by_index(&mut self, from: usize, to: usize) {
        let c = self.contacts_and_groups.remove(from);
        self.contacts_and_groups.insert(to, c);
    }

    pub fn selected(&self) -> Option<&Contact> {
        self.state
            .selected()
            .and_then(|i| self.contacts_and_groups.get(i))
    }
}

impl FromIterator<Contact> for Contacts {
    fn from_iter<T: IntoIterator<Item = Contact>>(iter: T) -> Self {
        let mut msgs = Self::default();
        msgs.extend(iter);
        msgs
    }
}

impl Extend<Contact> for Contacts {
    fn extend<T: IntoIterator<Item = Contact>>(&mut self, iter: T) {
        for contact in iter {
            if let ContactId::User(id) = &contact.id {
                self.contacts_by_id.insert(id.clone(), contact.clone());
            }
            self.contacts_and_groups.push(contact);
        }
    }
}
