use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use rstate::{
    Persisted, Persistence, SnapshotCodec, SnapshotStorage, StoreConfig, StoreError, StoreResult,
    Update,
};

type Hook = dyn Fn();

#[derive(Debug)]
pub struct Context {
    pub value: i64,
    pub local: Rc<Cell<bool>>,
}

pub enum Event {
    Add(i64),
    Keep,
    Fail,
}

#[derive(Clone, Default)]
pub struct Storage {
    pub values: Rc<RefCell<BTreeMap<String, Vec<u8>>>>,
    pub calls: Rc<RefCell<Vec<String>>>,
    pub fail_read: Rc<Cell<bool>>,
    pub fail_write: Rc<Cell<bool>>,
    pub fail_clear: Rc<Cell<bool>>,
    pub on_write: Rc<RefCell<Option<Rc<Hook>>>>,
    pub on_clear: Rc<RefCell<Option<Rc<Hook>>>>,
}

impl Storage {
    pub fn seed(&self, value: &str) {
        self.values
            .borrow_mut()
            .insert("state".into(), value.as_bytes().to_vec());
    }

    pub fn saved(&self) -> Option<String> {
        self.values
            .borrow()
            .get("state")
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }
}

impl SnapshotStorage for Storage {
    fn read(&self, name: &str) -> StoreResult<Option<Vec<u8>>> {
        self.calls.borrow_mut().push(format!("read:{name}"));
        if self.fail_read.get() {
            return Err(io_error("read failed"));
        }
        Ok(self.values.borrow().get(name).cloned())
    }

    fn write(&self, name: &str, bytes: &[u8]) -> StoreResult<()> {
        self.calls
            .borrow_mut()
            .push(format!("write:{}", String::from_utf8_lossy(bytes)));
        let hook = self.on_write.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        if self.fail_write.get() {
            return Err(io_error("write failed"));
        }
        self.values.borrow_mut().insert(name.into(), bytes.to_vec());
        Ok(())
    }

    fn remove(&self, name: &str) -> StoreResult<()> {
        self.calls.borrow_mut().push(format!("clear:{name}"));
        let hook = self.on_clear.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        if self.fail_clear.get() {
            return Err(io_error("clear failed"));
        }
        self.values.borrow_mut().remove(name);
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct Codec {
    pub fail_encode: Rc<Cell<bool>>,
    pub on_encode: Rc<RefCell<Option<Rc<Hook>>>>,
}

impl SnapshotCodec<Context> for Codec {
    fn encode(&self, context: &Context, version: u64) -> StoreResult<Vec<u8>> {
        let hook = self.on_encode.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        if self.fail_encode.get() {
            return Err(StoreError::new("encoding failed"));
        }
        Ok(format!("{version}:{}", context.value).into_bytes())
    }

    fn decode(&self, bytes: &[u8]) -> StoreResult<Persisted<Context>> {
        let text = std::str::from_utf8(bytes).map_err(StoreError::from_error)?;
        let (version, value) = text
            .split_once(':')
            .ok_or_else(|| StoreError::new("invalid persisted context"))?;
        Ok(Persisted {
            context: Context {
                value: value.parse().map_err(StoreError::from_error)?,
                local: Rc::new(Cell::new(false)),
            },
            version: version.parse().map_err(StoreError::from_error)?,
        })
    }
}

pub fn config(value: i64, effects: Rc<RefCell<Vec<String>>>) -> StoreConfig<Context, Event> {
    StoreConfig::new(
        Context {
            value,
            local: Rc::new(Cell::new(false)),
        },
        move |context, event, enqueue| match event {
            Event::Add(by) => {
                let effects = effects.clone();
                enqueue.effect(move |_| {
                    effects.borrow_mut().push("effect".into());
                    Ok(())
                });
                Ok(Update::replace(Context {
                    value: context
                        .value
                        .checked_add(*by)
                        .ok_or_else(|| StoreError::new("counter overflow"))?,
                    local: context.local.clone(),
                }))
            }
            Event::Keep => Ok(Update::Keep),
            Event::Fail => Err(StoreError::new("reducer failed")),
        },
    )
}

pub fn persistence(storage: &Storage, codec: &Codec) -> Persistence<Context> {
    Persistence::new("state", 1, storage.clone(), codec.clone())
}

fn io_error(message: &str) -> StoreError {
    StoreError::from_error(std::io::Error::other(message))
}
