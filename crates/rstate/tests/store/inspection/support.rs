use std::{cell::RefCell, rc::Rc};

use rstate::{Inspection, Store, StoreConfig, StoreError, Update};

pub(super) type Trace = Rc<RefCell<Vec<String>>>;
pub(super) type Fixture = Store<Context, Event, &'static str>;

pub(super) struct Context {
    pub count: i32,
    pub local: Rc<()>,
}

pub(super) enum Event {
    Add(i32),
    Keep,
    Same,
    Unknown,
    Start,
    Child,
    Fail,
    EffectFail,
}

pub(super) fn store(trace: &Trace) -> Fixture {
    let effects = trace.clone();
    Store::new(StoreConfig::new(
        Context {
            count: 0,
            local: Rc::new(()),
        },
        move |context: &Rc<Context>, event: &Event, enqueue| {
            let by = match event {
                Event::Add(by) => *by,
                Event::Keep | Event::Unknown => return Ok(Update::Keep),
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
                    enqueue.trigger(Event::Child);
                    enqueue.emit("parent");
                    1
                }
                Event::Child => {
                    enqueue.emit("child");
                    1
                }
                Event::EffectFail => {
                    enqueue.effect(|_| Err(StoreError::new("effect failed")));
                    1
                }
            };
            Ok(Update::replace(Context {
                count: context.count.saturating_add(by),
                local: context.local.clone(),
            }))
        },
    ))
}

pub(super) fn describe(inspection: &Inspection<'_, Context, Event>) -> String {
    match inspection {
        Inspection::Initial { snapshot } => format!("init:{}", snapshot.context.count),
        Inspection::Transition { event, snapshot } => {
            let name = match event {
                Event::Add(by) => format!("add:{by}"),
                Event::Keep => "keep".into(),
                Event::Same => "same".into(),
                Event::Unknown => "unknown".into(),
                Event::Start => "start".into(),
                Event::Child => "child".into(),
                Event::Fail => "fail".into(),
                Event::EffectFail => "effectFail".into(),
            };
            format!("inspect:{name}:{}", snapshot.context.count)
        }
    }
}
