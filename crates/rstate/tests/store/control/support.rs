use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, StoreConfig, StoreError, Update};

pub(super) type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Debug)]
pub(super) struct Context {
    pub count: i32,
    pub label: String,
    pub local: Rc<()>,
}

#[derive(Debug)]
pub(super) enum Event {
    Add(i32),
    Keep,
    Same,
    Start,
    Child,
    Fail,
}

pub(super) fn config(initial: i32, trace: &Trace) -> StoreConfig<Context, Event, &'static str> {
    let effects = trace.clone();
    StoreConfig::new(
        Context {
            count: initial,
            label: "initial".into(),
            local: Rc::new(()),
        },
        move |context: &Rc<Context>, event: &Event, enqueue| {
            let by = match event {
                Event::Add(by) => *by,
                Event::Keep => return Ok(Update::Keep),
                Event::Same => return Ok(Update::Replace(context.clone())),
                Event::Fail => return Err(StoreError::new("reducer failed")),
                Event::Start => {
                    let trace = effects.clone();
                    enqueue.effect(move |scope| {
                        trace
                            .borrow_mut()
                            .push(format!("effect:{}", scope.get_snapshot().context.count));
                        Ok(())
                    });
                    enqueue.emit("message");
                    enqueue.trigger(Event::Child);
                    1
                }
                Event::Child => 1,
            };
            Ok(Update::replace(Context {
                count: context.count.saturating_add(by),
                label: "changed".into(),
                local: context.local.clone(),
            }))
        },
    )
}

pub(super) fn describe(record: Inspection<'_, Context, Event>) -> String {
    match record {
        Inspection::Initial { snapshot } => format!("init:{}", snapshot.context.count),
        Inspection::Transition { event, snapshot } => {
            format!("event:{event:?}:{}", snapshot.context.count)
        }
        Inspection::Control { action, snapshot } => {
            format!("control:{action:?}:{}", snapshot.context.count)
        }
    }
}
