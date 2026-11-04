use std::{cell::RefCell, fmt, rc::Rc};

use rstate::{Store, StoreConfig, Update};

type DebugCallback = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

struct Context {
    count: i32,
    on_debug: DebugCallback,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let callback = self.on_debug.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        f.debug_struct("Context")
            .field("count", &self.count)
            .finish()
    }
}

struct Event;
struct Message;

#[test]
fn store_debug_releases_internal_borrows_before_formatting_context() {
    let on_debug = Rc::new(RefCell::new(None));
    let store = Store::<Context, Event, Message>::new(StoreConfig::new(
        Context {
            count: 0,
            on_debug: on_debug.clone(),
        },
        |context, _, _| {
            Ok(Update::replace(Context {
                count: context.count + 1,
                on_debug: context.on_debug.clone(),
            }))
        },
    ));
    let nested = store.clone();
    on_debug.replace(Some(Box::new(move || {
        nested.send(Event).unwrap();
    })));

    let debug = format!("{store:#?}");

    assert!(debug.contains("Store"));
    assert!(debug.contains("count: 0"));
    assert_eq!(store.get_snapshot().context.count, 1);
}
