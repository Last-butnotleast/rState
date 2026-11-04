use std::rc::Rc;

use rstate::{Effect, Snapshot, StoreError, StoreResult, Transition, Update};

#[derive(PartialEq, Eq)]
enum Phase {
    Pending,
    Running,
}

struct Job {
    phase: Phase,
}

enum Event {
    Start,
    Fail,
}

struct Table {
    row: Rc<Snapshot<Job>>,
    commands: Vec<&'static str>,
}

impl Table {
    fn transaction(
        &mut self,
        transition: &Transition<Job, Event, &'static str>,
        event: Event,
        commit: bool,
    ) -> StoreResult<bool> {
        let loaded = self.row.clone();
        let result = transition.transition(&loaded, event)?;
        if !result.allowed {
            return Ok(false);
        }
        let commands: Vec<_> = result
            .effects
            .iter()
            .map(|effect| match effect {
                Effect::Emit(message) => Ok(**message),
                Effect::Run(_) => Err(StoreError::new("transaction requires data outputs")),
            })
            .collect::<StoreResult<_>>()?;
        if !commit {
            return Err(StoreError::new("transaction aborted"));
        }
        self.row = result.snapshot;
        self.commands.extend(commands);
        Ok(true)
    }
}

#[test]
fn transaction_owns_state_and_only_commits_successful_proposals() {
    let transition = Transition::new(|job: &Rc<Job>, event: &Event, enqueue| match event {
        Event::Start if job.phase == Phase::Pending => {
            enqueue.emit("schedule-job");
            Ok(Update::replace(Job {
                phase: Phase::Running,
            }))
        }
        Event::Start => Ok(Update::Keep),
        Event::Fail => {
            enqueue.emit("discarded-command");
            Err(StoreError::new("application rejected event"))
        }
    });
    let original = Rc::new(Snapshot::new(Rc::new(Job {
        phase: Phase::Pending,
    })));
    let mut table = Table {
        row: original.clone(),
        commands: Vec::new(),
    };

    assert!(table.transaction(&transition, Event::Fail, true).is_err());
    assert!(table.transaction(&transition, Event::Start, false).is_err());
    assert!(Rc::ptr_eq(&table.row, &original));
    assert!(table.commands.is_empty());

    assert!(table.transaction(&transition, Event::Start, true).unwrap());
    assert!(table.row.context.phase == Phase::Running);
    assert_eq!(table.commands, ["schedule-job"]);
    let committed = table.row.clone();
    assert!(!table.transaction(&transition, Event::Start, true).unwrap());
    assert!(Rc::ptr_eq(&table.row, &committed));
    assert_eq!(table.commands, ["schedule-job"]);
    assert!(original.context.phase == Phase::Pending);
}
