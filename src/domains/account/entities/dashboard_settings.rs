use std::collections::BTreeMap;

use serde_json::{Map, Value};

pub enum DashboardSettingsUpdate {
    Replace(Value),
    Patch(BTreeMap<String, Value>),
}

impl DashboardSettingsUpdate {
    pub fn apply(self, current: &mut Value) {
        match self {
            Self::Replace(value) => *current = value,
            Self::Patch(fields) => merge_patch(current, Value::Object(fields.into_iter().collect())),
        }
    }
}

// Object merge semantics follow RFC 7396, including for legacy scalar documents.
fn merge_patch(current: &mut Value, patch: Value) {
    match patch {
        Value::Object(fields) => {
            let mut target = match std::mem::take(current) {
                Value::Object(object) => object,
                _ => Map::new(),
            };
            for (key, value) in fields {
                if value.is_null() {
                    target.remove(&key);
                } else {
                    merge_patch(target.entry(key).or_insert(Value::Null), value);
                }
            }
            *current = Value::Object(target);
        }
        value => *current = value,
    }
}
