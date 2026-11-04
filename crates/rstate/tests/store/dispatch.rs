use std::{cell::RefCell, rc::Rc};

use rstate::{Store, StoreConfig, Update};

use crate::support::{Event, counter};

#[test]
fn send_commits_and_notifies_synchronously_before_returning() {
    let store = counter();
    let trace = Rc::new(RefCell::new(vec!["before".to_string()]));
    let observed = Rc::clone(&trace);
    let current = store.clone();
    let _subscription = store.subscribe(move |snapshot| {
        assert!(Rc::ptr_eq(&snapshot, &current.get_snapshot()));
        observed
            .borrow_mut()
            .push(format!("callback:{}", snapshot.context.count));
        Ok(())
    });

    store.send(Event::Add(2)).unwrap();
    trace.borrow_mut().push("after".to_string());

    assert_eq!(*trace.borrow(), ["before", "callback:2", "after"]);
}

#[test]
fn context_updates_replace_the_whole_context() {
    struct Context {
        count: i32,
        label: Option<&'static str>,
    }
    let store = Store::<Context, ()>::new(StoreConfig::new(
        Context {
            count: 0,
            label: Some("initial"),
        },
        |_, (), _| {
            Ok(Update::replace(Context {
                count: 1,
                label: None,
            }))
        },
    ));

    store.send(()).unwrap();

    assert_eq!(store.get_snapshot().context.count, 1);
    assert_eq!(store.get_snapshot().context.label, None);
}
