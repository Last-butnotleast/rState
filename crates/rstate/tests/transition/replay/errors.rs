use std::{cell::Cell, error::Error, rc::Rc};

use rstate::{
    ReplayPolicy, Schema, SchemaCheck, SchemaMetadata, Schemas, Snapshot, Store, StoreConfig,
    StoreError, Transition, Update, ValidationOptions, replay,
};

#[test]
fn operational_failures_abort_replay_without_executing_collected_effects_or_consuming_more_events()
{
    let runs = Rc::new(Cell::new(0_u32));
    let effect_runs = runs.clone();
    let nested: Store<(), ()> = Store::new(StoreConfig::new((), |_, (), _| {
        Err(StoreError::from_error(std::io::Error::other("failed")))
    }));
    let transition: Transition<i32, i32> =
        Transition::new(move |context: &Rc<i32>, event: &i32, enqueue| {
            if *event == -1 {
                nested.send(())?;
            }
            let runs = effect_runs.clone();
            enqueue.effect(move |_| {
                runs.set(runs.get().saturating_add(1));
                Ok(())
            });
            Ok(Update::replace(context.saturating_add(*event)))
        });
    let loaded = Rc::new(Snapshot::new(Rc::new(1)));
    let requested = Rc::new(Cell::new(0_u32));
    let counts = requested.clone();
    let events = [2, -1, 3].into_iter().inspect(move |_| {
        counts.set(counts.get().saturating_add(1));
    });

    let failure = replay(&transition, loaded.clone(), events, ReplayPolicy::Skip).unwrap_err();

    assert_eq!(requested.get(), 2);
    assert_eq!(*loaded.context, 1);
    assert_eq!(runs.get(), 0);
    assert_eq!(failure.dispatch_status(), None);
    assert!(failure.source().unwrap().is::<std::io::Error>());
}

#[test]
fn schema_provider_failures_remain_operational_errors_under_skip() {
    let transition: Transition<(), i32> = Transition::new(|_, _, _| Ok(Update::Keep))
        .with_schemas(Schemas::default().with_events(Schema::new(
            SchemaMetadata::new("events"),
            |event: &i32| {
                if *event == 2 {
                    Err(StoreError::from_error(std::io::Error::other(
                        "validator unavailable",
                    )))
                } else {
                    Ok(SchemaCheck::Valid)
                }
            },
        )))
        .with_validation(ValidationOptions::default());
    let loaded = Rc::new(Snapshot::new(Rc::new(())));
    let failure = replay(&transition, loaded, [1, 2, 3], ReplayPolicy::Skip).unwrap_err();

    assert_eq!(failure.dispatch_status(), None);
    assert!(failure.validation_error().is_none());
    assert_eq!(
        failure.source().unwrap().to_string(),
        "validator unavailable"
    );
}
