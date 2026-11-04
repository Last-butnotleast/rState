use std::cell::Cell;
use std::rc::Rc;

use rstate::Selection;
use serde_json::{Value, json};
use wasm_bindgen::JsValue;

use super::super::selection::value::{LabelValue, NumberValue};
use crate::scenarios::{self, Fixture};

#[derive(Default)]
pub struct Counts {
    source: Cell<u32>,
    left: Cell<u32>,
    right: Cell<u32>,
    combined: Cell<u32>,
    chain: Cell<u32>,
}

impl Counts {
    pub fn snapshot(&self) -> Value {
        json!({
            "source": self.source.get(),
            "left": self.left.get(),
            "right": self.right.get(),
            "combined": self.combined.get(),
            "chain": self.chain.get(),
        })
    }
}

pub struct Graph {
    pub left_store: Fixture,
    pub right_store: Fixture,
    pub left: Selection<NumberValue>,
    pub right: Selection<NumberValue>,
    pub combined: Selection<NumberValue>,
    pub chain: Selection<LabelValue>,
    pub counts: Rc<Counts>,
}

impl Graph {
    pub fn new(mode: &str) -> Result<Self, JsValue> {
        if !matches!(mode, "independent" | "diamond" | "duplicate") {
            return Err(JsValue::from_str("unknown combination mode"));
        }
        let counts = Rc::new(Counts::default());
        let left_store = scenarios::create("counter", Rc::new(|_| Ok(())));
        let right_store = if mode == "independent" {
            scenarios::create("counter", Rc::new(|_| Ok(())))
        } else {
            left_store.clone()
        };
        let (left, right) = if mode == "independent" {
            (
                selection(&left_store, counts.clone(), |counts| &counts.left),
                selection(&right_store, counts.clone(), |counts| &counts.right),
            )
        } else {
            let source = selection(&left_store, counts.clone(), |counts| &counts.source);
            if mode == "duplicate" {
                (source.clone(), source)
            } else {
                let left_counts = counts.clone();
                let right_counts = counts.clone();
                (
                    source.map(
                        move |value| {
                            increment(&left_counts.left);
                            NumberValue {
                                value: value.value.saturating_mul(2),
                            }
                        },
                        equal,
                    ),
                    source.map(
                        move |value| {
                            increment(&right_counts.right);
                            NumberValue {
                                value: value.value.saturating_add(1),
                            }
                        },
                        equal,
                    ),
                )
            }
        };
        let combined_counts = counts.clone();
        let combined = left.combine(
            &right,
            move |left, right| {
                increment(&combined_counts.combined);
                NumberValue {
                    value: left.value.saturating_add(right.value).div_euclid(2),
                }
            },
            equal,
        );
        let chain_counts = counts.clone();
        let chain = combined.map(
            move |value| {
                increment(&chain_counts.chain);
                LabelValue {
                    value: format!("combined:{}", value.value),
                }
            },
            |left, right| left.value == right.value,
        );
        Ok(Self {
            left_store,
            right_store,
            left,
            right,
            combined,
            chain,
            counts,
        })
    }
}

fn selection(
    store: &Fixture,
    counts: Rc<Counts>,
    counter: fn(&Counts) -> &Cell<u32>,
) -> Selection<NumberValue> {
    store.select(
        move |context| {
            increment(counter(&counts));
            NumberValue {
                value: context.count,
            }
        },
        equal,
    )
}

const fn equal(left: &NumberValue, right: &NumberValue) -> bool {
    left.value == right.value
}

fn increment(counter: &Cell<u32>) {
    counter.set(counter.get().saturating_add(1));
}
