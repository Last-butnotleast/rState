use gpui::{App, Context};
use rstate::{JsonCodec, Persistence, SnapshotStorage, Store, StoreConfig, StoreResult, Update};
use rstate_gpui::{
    CommandPolicy, CommandScope, CommandSubmitError, PersistenceBinding, SelectionBinding,
    ViewBindings, dispatch, handle_result,
};

struct Storage;

impl SnapshotStorage for Storage {
    fn read(&self, _name: &str) -> StoreResult<Option<Vec<u8>>> {
        Ok(None)
    }

    fn write(&self, _name: &str, _bytes: &[u8]) -> StoreResult<()> {
        Ok(())
    }

    fn remove(&self, _name: &str) -> StoreResult<()> {
        Ok(())
    }
}

struct ConsumerView {
    value: SelectionBinding<i32>,
    can_undo: SelectionBinding<bool>,
    persistence: PersistenceBinding,
}

impl ConsumerView {
    fn new(store: &Store<i32, i32>, cx: &Context<Self>) -> StoreResult<Self> {
        let bindings = ViewBindings::new(cx);
        Ok(Self {
            value: bindings.bind(store.select(|value| *value, PartialEq::eq)),
            can_undo: bindings.bind(store.select_snapshot(
                |snapshot| snapshot.history_status().past != 0,
                PartialEq::eq,
            )),
            persistence: bindings.bind_persistence(store)?,
        })
    }

    fn values(&self) -> (i32, bool, u64) {
        (
            *self.value.get(),
            *self.can_undo.get(),
            self.persistence.get().sequence(),
        )
    }
}

fn commands(cx: &App) -> Result<(), CommandSubmitError> {
    let scope = CommandScope::new(cx);
    let channel = scope.channel(CommandPolicy::Latest);
    let ticket = channel.submit(
        |_cx| async { 1_u32 },
        |value, _cx| {
            assert_eq!(value, 1);
            Ok(())
        },
        |_error, _cx| {},
    )?;
    ticket.cancel();
    channel.submit_background(
        || async { 2_u32 },
        |value, _cx| {
            assert_eq!(value, 2);
            Ok(())
        },
        |_error, _cx| {},
    )?;
    channel.invalidate();
    scope.invalidate();
    scope.close();
    Ok(())
}

fn main() -> StoreResult<()> {
    let store: Store<i32, i32> = Store::new_persisted(
        StoreConfig::new(0, |_, event, _| Ok(Update::replace(*event))).with_history(4),
        Persistence::new("consumer", 1, Storage, JsonCodec::new()),
    )?;
    let _persistence = store
        .subscribe_persistence(|status| {
            assert!(status.sequence() > 0);
        })?
        .scoped();
    store.send(1)?;
    assert!(dispatch(&store, 2, |error| eprintln!("{error}")));
    assert!(handle_result(store.undo(), |error| eprintln!("{error}")));
    assert_eq!(*store.get_snapshot().context, 1);
    store.save()?;
    store.clear_storage()?;
    assert!(store.persistence_status().is_some());
    let _constructor: fn(&Store<i32, i32>, &Context<ConsumerView>) -> StoreResult<ConsumerView> =
        ConsumerView::new;
    let _reader: fn(&ConsumerView) -> (i32, bool, u64) = ConsumerView::values;
    let _commands: fn(&App) -> Result<(), CommandSubmitError> = commands;
    Ok(())
}
