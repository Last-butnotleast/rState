use std::{cell::Cell, rc::Rc};

use rstate::{Effect, Snapshot, Transition, Update};

#[derive(PartialEq, Eq)]
enum Command {
    Schedule(u64),
    Cancel(u64),
}

enum Event {
    Start,
    Child(u64),
}

#[test]
fn emitted_commands_preserve_fifo_order_and_shared_non_clone_values() {
    let transition = Transition::new(|_: &Rc<()>, event: &Event, enqueue| {
        match event {
            Event::Start => {
                enqueue.emit(Command::Schedule(1));
                enqueue.trigger(Event::Child(2));
                enqueue.trigger(Event::Child(3));
                enqueue.emit(Command::Cancel(4));
            }
            Event::Child(id) => enqueue.emit(Command::Schedule(*id)),
        }
        Ok(Update::Keep)
    });
    let snapshot = Rc::new(Snapshot::new(Rc::new(())));
    let proposal = transition.transition(&snapshot, Event::Start).unwrap();
    let Effect::Emit(first) = proposal.effects.first().unwrap() else {
        panic!("expected command data");
    };
    let retained = first.clone();
    let result = proposal.into_commands().unwrap();

    assert!(Rc::ptr_eq(&result.snapshot, &snapshot));
    assert!(result.allowed);
    assert!(Rc::ptr_eq(result.commands.first().unwrap(), &retained));
    let expected = [
        Command::Schedule(1),
        Command::Cancel(4),
        Command::Schedule(2),
        Command::Schedule(3),
    ];
    assert!(result.commands.iter().map(Rc::as_ref).eq(expected.iter()));
    assert!(format!("{result:?}").contains("command_count: 4"));
}

struct Capture(Rc<Cell<u32>>);

impl Drop for Capture {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_add(1));
    }
}

#[test]
fn mixed_effects_return_the_complete_original_proposal_without_releasing_callbacks() {
    let runs = Rc::new(Cell::new(0_u32));
    let drops = Rc::new(Cell::new(0_u32));
    let effect_runs = runs.clone();
    let captures = drops.clone();
    let transition = Transition::new(move |context: &Rc<()>, (): &(), enqueue| {
        enqueue.emit(Command::Schedule(1));
        let capture = Capture(captures.clone());
        let runs = effect_runs.clone();
        enqueue.effect(move |_| {
            runs.set(runs.get().saturating_add(1));
            drop(capture);
            Ok(())
        });
        enqueue.emit(Command::Cancel(2));
        Ok(Update::Replace(context.clone()))
    });
    let snapshot = Rc::new(Snapshot::new(Rc::new(())));
    let result = transition.transition(&snapshot, ()).unwrap();
    let retained: Vec<_> = result
        .effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Emit(message) => Some(message.clone()),
            Effect::Run(_) => None,
        })
        .collect();

    let recovered = result.into_commands().unwrap_err();
    assert!(Rc::ptr_eq(&recovered.snapshot, &snapshot));
    assert!(recovered.allowed);
    assert_eq!(runs.get(), 0);
    assert_eq!(drops.get(), 0);
    let mut actual = recovered.effects.iter();
    assert!(
        matches!(actual.next(), Some(Effect::Emit(message)) if Rc::ptr_eq(message, retained.first().unwrap()))
    );
    assert!(matches!(actual.next(), Some(Effect::Run(_))));
    assert!(
        matches!(actual.next(), Some(Effect::Emit(message)) if Rc::ptr_eq(message, retained.last().unwrap()))
    );
    assert!(actual.next().is_none());
    drop(recovered);
    assert_eq!(runs.get(), 0);
    assert_eq!(drops.get(), 1);
}

#[test]
fn unavailable_results_extract_empty_commands_without_changing_availability() {
    let transition: Transition<(), ()> = Transition::new(|_, (), _| Ok(Update::Keep));
    let snapshot = Rc::new(Snapshot::new(Rc::new(())));
    let result = transition
        .transition(&snapshot, ())
        .unwrap()
        .into_commands()
        .unwrap();

    assert!(Rc::ptr_eq(&result.snapshot, &snapshot));
    assert!(result.commands.is_empty());
    assert!(!result.allowed);
}

#[cfg(feature = "json")]
#[test]
fn applications_serialize_and_restore_command_enums_without_rc_serde_support() {
    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    enum SerializableCommand {
        ScheduleJob { job_id: u64 },
        ScheduleTimer { delay_ms: u64 },
    }

    let transition = Transition::new(|_: &Rc<()>, (): &(), enqueue| {
        enqueue.emit(SerializableCommand::ScheduleJob { job_id: 42 });
        enqueue.emit(SerializableCommand::ScheduleTimer { delay_ms: 100 });
        Ok(Update::Keep)
    });
    let snapshot = Rc::new(Snapshot::new(Rc::new(())));
    let result = transition
        .transition(&snapshot, ())
        .unwrap()
        .into_commands()
        .unwrap();
    let commands: Vec<_> = result.commands.iter().map(Rc::as_ref).collect();
    let encoded = serde_json::to_string(&commands).unwrap();
    let decoded: Vec<SerializableCommand> = serde_json::from_str(&encoded).unwrap();

    assert!(commands.into_iter().eq(decoded.iter()));
    assert_eq!(
        decoded,
        [
            SerializableCommand::ScheduleJob { job_id: 42 },
            SerializableCommand::ScheduleTimer { delay_ms: 100 }
        ]
    );
}
