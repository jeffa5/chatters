use std::collections::BTreeMap;

use presage::store::Thread;
use uuid::Uuid;

use crate::backends::Contact;

#[derive(Debug, Default)]
pub struct Contacts {
    contacts_and_groups: Vec<Contact>,
    contacts_by_id: BTreeMap<Uuid, Contact>,
}

impl Contacts {
    pub fn new(contacts_and_groups: Vec<Contact>) -> Self {
        let contacts_by_id = contacts_and_groups
            .iter()
            .filter_map(|c| {
                if let Thread::Contact(uuid) = c.thread_id {
                    Some((uuid, c.clone()))
                } else {
                    None
                }
            })
            .collect();
        Self {
            contacts_and_groups,
            contacts_by_id,
        }
    }

    pub fn contact_or_group_by_index(&self, index: usize) -> Option<&Contact> {
        self.contacts_and_groups.get(index)
    }

    pub fn contact_or_group_by_index_mut(&mut self, index: usize) -> Option<&mut Contact> {
        self.contacts_and_groups.get_mut(index)
    }

    pub fn contact_by_id(&self, id: &Uuid) -> Option<&Contact> {
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
}
