use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Context {
    pub count: i64,
    pub items: Vec<i64>,
    pub total: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    Inc {
        #[serde(default)]
        by: i64,
    },
    Bump {
        #[serde(default)]
        by: i64,
    },
    Keep,
    Same,
    Equal,
    Add {
        #[serde(default)]
        price: i64,
    },
    Recalculate,
    Receipt,
    Start,
    Fail,
    EffectFail,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    Receipt { total: i64 },
}
