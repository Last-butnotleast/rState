use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};

pub struct NumberValue {
    pub value: i64,
}

pub struct LabelValue {
    pub value: String,
}

#[derive(Default)]
pub struct Counts {
    pub source: Cell<u32>,
    pub mapped: Cell<u32>,
    pub chain: Cell<u32>,
}

impl Counts {
    pub fn snapshot(&self) -> Value {
        json!({
            "source": self.source.get(),
            "mapped": self.mapped.get(),
            "chain": self.chain.get(),
        })
    }
}

pub struct Identities<T> {
    values: Vec<Rc<T>>,
}

impl<T> Default for Identities<T> {
    fn default() -> Self {
        Self { values: Vec::new() }
    }
}

impl<T> Identities<T> {
    pub fn identify(&mut self, value: Rc<T>) -> usize {
        if let Some(index) = self.values.iter().position(|item| Rc::ptr_eq(item, &value)) {
            return index.saturating_add(1);
        }
        self.values.push(value);
        self.values.len()
    }
}
