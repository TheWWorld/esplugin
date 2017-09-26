/*
 * This file is part of libespm
 *
 * Copyright (C) 2017 Oliver Hamlet
 *
 * libespm is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * libespm is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with libespm. If not, see <http://www.gnu.org/licenses/>.
 */
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::string::ToString;

use unicase::{eq, UniCase};

#[derive(Clone, Debug, Default)]
pub struct FormId {
    pub object_index: u32,
    pub plugin_name: String,
}

impl FormId {
    pub fn new<T: AsRef<str> + ToString>(
        parent_plugin_name: &str,
        masters: &[T],
        raw_form_id: u32,
    ) -> FormId {
        let mod_index = (raw_form_id >> 24) as usize;

        FormId {
            object_index: raw_form_id & 0xFFFFFF,
            plugin_name: masters
                .get(mod_index)
                .map_or(parent_plugin_name, |m| m.as_ref())
                .to_string(),
        }
    }
}

impl Ord for FormId {
    fn cmp(&self, other: &FormId) -> Ordering {
        if self.object_index != other.object_index {
            self.object_index.cmp(&other.object_index)
        } else {
            UniCase::new(&self.plugin_name).cmp(&UniCase::new(&other.plugin_name))
        }
    }
}

impl PartialOrd for FormId {
    fn partial_cmp(&self, other: &FormId) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FormId {
    fn eq(&self, other: &FormId) -> bool {
        self.object_index == other.object_index && eq(&self.plugin_name, &other.plugin_name)
    }
}

impl Eq for FormId {}

impl Hash for FormId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.object_index.hash(state);
        self.plugin_name.to_lowercase().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PARENT_PLUGIN_NAME: &'static str = "plugin0";
    const OTHER_PLUGIN_NAME: &'static str = "plugin1";
    const MASTERS: &'static [&'static str] = &["plugin2", "plugin3"];
    const NO_MASTERS: &'static [&'static str] = &[];

    #[test]
    fn object_index_should_equal_last_three_bytes_of_raw_form_id_value() {
        let form_id = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);

        assert_eq!(0x01, form_id.object_index);

        let form_id = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01000001);

        assert_eq!(0x01, form_id.object_index);
    }

    #[test]
    fn should_store_master_at_mod_index_as_plugin_name() {
        let form_id = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);

        assert_eq!(MASTERS[0], form_id.plugin_name);

        let form_id = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01000001);

        assert_eq!(MASTERS[1], form_id.plugin_name);
    }

    #[test]
    fn should_store_parent_plugin_name_for_mod_index_greater_than_last_index_of_masters() {
        let form_id = FormId::new(PARENT_PLUGIN_NAME, NO_MASTERS, 0x01);

        assert_eq!(PARENT_PLUGIN_NAME, form_id.plugin_name);

        let form_id = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x05000001);

        assert_eq!(PARENT_PLUGIN_NAME, form_id.plugin_name);
    }

    #[test]
    fn form_ids_should_not_be_equal_if_plugin_names_are_unequal() {
        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);
        let form_id2 = FormId::new(OTHER_PLUGIN_NAME, MASTERS, 0x05000001);

        assert_ne!(form_id1, form_id2);
    }

    #[test]
    fn form_ids_should_not_be_equal_if_object_indices_are_unequal() {
        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);
        let form_id2 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x02);

        assert_ne!(form_id1, form_id2);
    }

    #[test]
    fn form_ids_with_equal_plugin_names_and_object_ids_should_be_equal() {
        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, NO_MASTERS, 0x01);
        let form_id2 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x05000001);

        assert_eq!(form_id1, form_id2);
    }

    #[test]
    fn form_ids_should_be_equal_if_plugin_names_are_case_insensitively_equal() {
        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);
        let form_id2 = FormId::new("PLUGIN0", MASTERS, 0x01);

        assert_eq!(form_id1, form_id2);
    }

    #[test]
    fn form_ids_should_be_ordered_according_to_object_index_then_icase_lexicographically() {
        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x05000001);
        let form_id2 = FormId::new("PLUGIN0", MASTERS, 0x05000001);

        assert_eq!(Ordering::Equal, form_id1.cmp(&form_id2));
        assert_eq!(Ordering::Equal, form_id2.cmp(&form_id1));

        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);
        let form_id2 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x02);

        assert_eq!(Ordering::Less, form_id1.cmp(&form_id2));
        assert_eq!(Ordering::Greater, form_id2.cmp(&form_id1));

        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x05000001);
        let form_id2 = FormId::new(OTHER_PLUGIN_NAME, MASTERS, 0x05000001);

        assert_eq!(Ordering::Less, form_id1.cmp(&form_id2));
        assert_eq!(Ordering::Greater, form_id2.cmp(&form_id1));
    }

    #[test]
    fn form_ids_with_case_insensitive_equal_plugin_names_should_have_the_same_hash() {
        let form_id1 = FormId::new(PARENT_PLUGIN_NAME, MASTERS, 0x01);
        let form_id2 = FormId::new("PLUGIN0", MASTERS, 0x01);

        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        form_id1.hash(&mut hasher);
        let hash1 = hasher.finish();

        let mut hasher = DefaultHasher::new();
        form_id2.hash(&mut hasher);
        let hash2 = hasher.finish();

        assert_eq!(hash1, hash2);
    }
}
