use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rstate::{
    Persisted, Persistence, SnapshotCodec, SnapshotStorage, Store, StoreError, StoreFactory,
    StoreResult, Update, ValidationOptions, ValidationStage,
};

use super::support::{Context, Event, Message, Trace, config, context, schemas, trace};

#[derive(Clone, Default)]
struct Storage {
    bytes: Rc<RefCell<Option<Vec<u8>>>>,
    calls: Trace,
}

impl Storage {
    fn seed(&self, value: &str) {
        self.bytes.replace(Some(value.as_bytes().to_vec()));
    }

    fn saved(&self) -> Option<String> {
        self.bytes
            .borrow()
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }
}

impl SnapshotStorage for Storage {
    fn read(&self, name: &str) -> StoreResult<Option<Vec<u8>>> {
        self.calls.borrow_mut().push(format!("read:{name}"));
        Ok(self.bytes.borrow().clone())
    }

    fn write(&self, name: &str, bytes: &[u8]) -> StoreResult<()> {
        self.calls
            .borrow_mut()
            .push(format!("write:{name}:{}", String::from_utf8_lossy(bytes)));
        self.bytes.replace(Some(bytes.to_vec()));
        Ok(())
    }

    fn remove(&self, name: &str) -> StoreResult<()> {
        self.calls.borrow_mut().push(format!("remove:{name}"));
        self.bytes.take();
        Ok(())
    }
}

struct Codec;

impl SnapshotCodec<Context> for Codec {
    fn encode(&self, context: &Context, version: u64) -> StoreResult<Vec<u8>> {
        Ok(format!("{version}:{}", context.count).into_bytes())
    }

    fn decode(&self, bytes: &[u8]) -> StoreResult<Persisted<Context>> {
        let value = std::str::from_utf8(bytes).map_err(StoreError::from_error)?;
        let (version, count) = value
            .split_once(':')
            .ok_or_else(|| StoreError::new("invalid context bytes"))?;
        Ok(Persisted {
            version: version.parse().map_err(StoreError::from_error)?,
            context: context(count.parse().map_err(StoreError::from_error)?),
        })
    }
}

fn persistence(storage: &Storage) -> Persistence<Context> {
    Persistence::new("state", 1, storage.clone(), Codec)
}

#[test]
fn validated_persistence_checks_defaults_and_loaded_context_without_overwriting_data() {
    let trace = trace();
    let storage = Storage::default();
    storage.seed("1:5");
    let declared = schemas(&trace, Rc::new(Cell::new(0)));
    let invalid_defaults = Store::new_persisted_validated(
        config(-1, &trace).with_schemas(declared.clone()),
        persistence(&storage),
        ValidationOptions::default(),
    )
    .err()
    .unwrap();
    assert_eq!(
        invalid_defaults.validation_error().unwrap().stage,
        ValidationStage::InitialContext
    );
    assert!(storage.calls.borrow().is_empty());
    assert_eq!(storage.saved().as_deref(), Some("1:5"));
    assert_eq!(*trace.borrow(), ["context:-1"]);

    storage.seed("1:-1");
    trace.borrow_mut().clear();
    let invalid_loaded = Store::new_persisted_validated(
        config(0, &trace).with_schemas(declared.clone()),
        persistence(&storage),
        ValidationOptions::default(),
    )
    .err()
    .unwrap();
    assert_eq!(
        invalid_loaded.validation_error().unwrap().stage,
        ValidationStage::InitialContext
    );
    assert_eq!(*trace.borrow(), ["context:0", "context:-1"]);
    assert_eq!(*storage.calls.borrow(), ["read:state"]);
    assert_eq!(storage.saved().as_deref(), Some("1:-1"));

    trace.borrow_mut().clear();
    let ordinary = Store::new_persisted(
        config(0, &trace).with_schemas(declared),
        persistence(&storage),
    )
    .unwrap();
    assert_eq!(ordinary.get_snapshot().context.count, -1);
    assert!(trace.borrow().is_empty());
}

#[test]
fn validated_actions_and_factory_hydration_preserve_persistence_and_history_boundaries() {
    let trace = trace();
    let minimum = Rc::new(Cell::new(0));
    let storage = Storage::default();
    storage.seed("1:5");
    let store = Store::new_persisted_validated(
        config(0, &trace)
            .with_history(4)
            .with_schemas(schemas(&trace, minimum.clone())),
        persistence(&storage),
        ValidationOptions::default(),
    )
    .unwrap();
    assert_eq!(store.get_initial_snapshot().context.count, 5);
    assert_eq!(store.history_status().past, 0);
    storage.calls.borrow_mut().clear();
    let initial = store.get_snapshot();
    assert!(store.send(Event::Break).is_err());
    assert!(store.send(Event::Emit(-1)).is_err());
    assert!(store.send(Event::Add(-1)).is_err());
    assert!(Rc::ptr_eq(&initial, &store.get_snapshot()));
    assert_eq!(store.history_status().past, 0);
    assert!(storage.calls.borrow().is_empty());
    assert_eq!(storage.saved().as_deref(), Some("1:5"));

    store.send(Event::Add(1)).unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:6"));
    let committed = store.get_snapshot();
    let committed_status = store.history_status();
    storage.calls.borrow_mut().clear();
    assert!(store.can(Event::Add(1)).unwrap());
    let predicted = store.transition(committed.clone(), Event::Add(1)).unwrap();
    assert_eq!(predicted.snapshot.context.count, 7);
    assert!(Rc::ptr_eq(&committed, &store.get_snapshot()));
    assert_eq!(store.history_status(), committed_status);
    assert!(storage.calls.borrow().is_empty());

    minimum.set(6);
    assert!(store.undo().is_err());
    assert!(store.reset().is_err());
    assert!(Rc::ptr_eq(&committed, &store.get_snapshot()));
    assert!(storage.calls.borrow().is_empty());
    minimum.set(0);
    store.undo().unwrap();
    assert_eq!(storage.saved().as_deref(), Some("1:5"));

    let factory: StoreFactory<i32, Context, Event, Message> =
        StoreFactory::new(|count| Ok(context(count)), |_, _, _| Ok(Update::Keep))
            .with_schemas(schemas(&trace, minimum));
    let recreated = factory
        .create_persisted_validated(0, persistence(&storage), ValidationOptions::default())
        .unwrap();
    assert_eq!(recreated.get_snapshot().context.count, 5);
    assert_eq!(recreated.history_status().past, 0);
}
