//! Cross-component lifecycle contract tests (bd-1dg21).
//!
//! These integration tests capture the observable behavioral contract of the
//! runtime lifecycle that MUST be preserved during the Asupersync migration.
//! They test through public APIs only (ProgramSimulator, CancellationToken,
//! effect system metrics). Tests requiring internal APIs (StopSignal::new,
//! SubscriptionManager) live in the crate-internal unit test modules.

#![forbid(unsafe_code)]

use ftui_core::event::Event;
use ftui_render::frame::Frame;
use ftui_runtime::program::{Cmd, Model};
use ftui_runtime::simulator::ProgramSimulator;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

// ============================================================================
// Test model: LifecycleTracker
//
// Records the exact sequence of lifecycle events to verify ordering contracts.
// ============================================================================

struct LifecycleTracker {
    trace: Vec<String>,
}

#[derive(Debug)]
enum LMsg {
    Init,
    Tick,
    Quit,
    TaskResult(String),
    SpawnTask,
}

impl From<Event> for LMsg {
    fn from(_: Event) -> Self {
        LMsg::Tick
    }
}

impl Model for LifecycleTracker {
    type Message = LMsg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.trace.push("init".into());
        Cmd::msg(LMsg::Init)
    }

    fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
        match msg {
            LMsg::Init => {
                self.trace.push("update:init".into());
                Cmd::none()
            }
            LMsg::Tick => {
                self.trace.push("update:tick".into());
                Cmd::none()
            }
            LMsg::Quit => {
                self.trace.push("update:quit".into());
                Cmd::quit()
            }
            LMsg::TaskResult(s) => {
                self.trace.push(format!("update:task-result:{s}"));
                Cmd::none()
            }
            LMsg::SpawnTask => {
                self.trace.push("update:spawn-task".into());
                Cmd::task(|| LMsg::TaskResult("done".into()))
            }
        }
    }

    fn view(&self, _frame: &mut Frame) {}

    fn on_shutdown(&mut self) -> Cmd<Self::Message> {
        self.trace.push("on_shutdown".into());
        Cmd::none()
    }
}

// ============================================================================
// CONTRACT: Full lifecycle trace ordering
// ============================================================================

/// CONTRACT: The lifecycle must follow: init -> update(init_cmd) -> updates
/// -> quit -> on_shutdown. This ordering is critical for the Asupersync
/// migration to preserve.
#[test]
fn contract_lifecycle_ordering_init_update_shutdown() {
    let mut sim = ProgramSimulator::new(LifecycleTracker { trace: vec![] });

    sim.init();
    sim.send(LMsg::SpawnTask);
    sim.send(LMsg::Quit);

    // Manually invoke on_shutdown (simulator doesn't auto-call it;
    // the real runtime does it in the shutdown sequence).
    let _shutdown_cmd = sim.model_mut().on_shutdown();

    let trace = &sim.model().trace;
    assert_eq!(
        trace,
        &[
            "init",
            "update:init",
            "update:spawn-task",
            "update:task-result:done",
            "update:quit",
            "on_shutdown",
        ],
        "lifecycle must follow: init -> update(init_cmd) -> updates -> quit -> on_shutdown"
    );
}

/// CONTRACT: Cmd::Task results are routed through Model::update() synchronously
/// in the simulator, and the ordering is deterministic.
#[test]
fn contract_task_result_ordering_is_deterministic() {
    struct MultiTaskModel {
        trace: Vec<String>,
    }

    #[derive(Debug)]
    enum MTMsg {
        SpawnAll,
        Result(String),
    }

    impl From<Event> for MTMsg {
        fn from(_: Event) -> Self {
            MTMsg::SpawnAll
        }
    }

    impl Model for MultiTaskModel {
        type Message = MTMsg;

        fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
            match msg {
                MTMsg::SpawnAll => {
                    self.trace.push("spawn-all".into());
                    Cmd::batch(vec![
                        Cmd::task(|| MTMsg::Result("task-a".into())),
                        Cmd::task(|| MTMsg::Result("task-b".into())),
                        Cmd::task(|| MTMsg::Result("task-c".into())),
                    ])
                }
                MTMsg::Result(s) => {
                    self.trace.push(format!("result:{s}"));
                    Cmd::none()
                }
            }
        }

        fn view(&self, _frame: &mut Frame) {}
    }

    // Run twice and verify deterministic ordering
    for _ in 0..3 {
        let mut sim = ProgramSimulator::new(MultiTaskModel { trace: vec![] });
        sim.init();
        sim.send(MTMsg::SpawnAll);

        assert_eq!(
            sim.model().trace,
            vec![
                "spawn-all",
                "result:task-a",
                "result:task-b",
                "result:task-c",
            ],
            "task results must be processed in batch submission order"
        );
    }
}

/// CONTRACT: Cmd::Batch stops executing after Cmd::Quit.
#[test]
fn contract_batch_halts_on_quit() {
    struct HaltModel {
        steps: Vec<&'static str>,
    }

    #[derive(Debug)]
    enum HMsg {
        Step(&'static str),
        TriggerBatchWithQuit,
    }

    impl From<Event> for HMsg {
        fn from(_: Event) -> Self {
            HMsg::Step("event")
        }
    }

    impl Model for HaltModel {
        type Message = HMsg;

        fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
            match msg {
                HMsg::Step(s) => {
                    self.steps.push(s);
                    Cmd::none()
                }
                HMsg::TriggerBatchWithQuit => Cmd::batch(vec![
                    Cmd::msg(HMsg::Step("before-quit")),
                    Cmd::quit(),
                    Cmd::msg(HMsg::Step("after-quit")),
                ]),
            }
        }

        fn view(&self, _frame: &mut Frame) {}
    }

    let mut sim = ProgramSimulator::new(HaltModel { steps: vec![] });
    sim.init();
    sim.send(HMsg::TriggerBatchWithQuit);

    assert!(!sim.is_running());
    assert_eq!(
        sim.model().steps,
        vec!["before-quit"],
        "commands after Quit in a Batch must not execute"
    );
}

/// CONTRACT: Cmd::Sequence stops executing after Cmd::Quit.
#[test]
fn contract_sequence_halts_on_quit() {
    struct SeqModel {
        steps: Vec<&'static str>,
    }

    #[derive(Debug)]
    enum SMsg {
        Step(&'static str),
        TriggerSeqWithQuit,
    }

    impl From<Event> for SMsg {
        fn from(_: Event) -> Self {
            SMsg::Step("event")
        }
    }

    impl Model for SeqModel {
        type Message = SMsg;

        fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
            match msg {
                SMsg::Step(s) => {
                    self.steps.push(s);
                    Cmd::none()
                }
                SMsg::TriggerSeqWithQuit => Cmd::sequence(vec![
                    Cmd::msg(SMsg::Step("before-quit")),
                    Cmd::quit(),
                    Cmd::msg(SMsg::Step("after-quit")),
                ]),
            }
        }

        fn view(&self, _frame: &mut Frame) {}
    }

    let mut sim = ProgramSimulator::new(SeqModel { steps: vec![] });
    sim.init();
    sim.send(SMsg::TriggerSeqWithQuit);

    assert!(!sim.is_running());
    assert_eq!(
        sim.model().steps,
        vec!["before-quit"],
        "commands after Quit in a Sequence must not execute"
    );
}

// ============================================================================
// CONTRACT: CancellationToken integration
// ============================================================================

/// CONTRACT: CancellationToken must cooperatively stop background work.
/// cancel() must wake threads blocked in wait_timeout().
#[test]
fn contract_cancellation_token_stops_background_work() {
    use ftui_runtime::cancellation::CancellationSource;

    let source = CancellationSource::new();
    let token = source.token();
    let work_done = Arc::new(AtomicUsize::new(0));
    let work_clone = work_done.clone();

    let handle = std::thread::spawn(move || {
        while !token.is_cancelled() {
            work_clone.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    std::thread::sleep(Duration::from_millis(50));
    let before_cancel = work_done.load(Ordering::SeqCst);
    assert!(before_cancel > 0, "work should have started");

    source.cancel();
    handle.join().unwrap();

    let at_cancel = work_done.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(50));
    let after_wait = work_done.load(Ordering::SeqCst);
    assert_eq!(
        at_cancel, after_wait,
        "no work should happen after cancellation"
    );
}

/// CONTRACT: Dropping CancellationSource does NOT cancel the token.
/// Cancellation must be explicit.
#[test]
fn contract_cancellation_drop_does_not_cancel() {
    use ftui_runtime::cancellation::CancellationSource;

    let source = CancellationSource::new();
    let token = source.token();
    drop(source);
    assert!(
        !token.is_cancelled(),
        "dropping source must not cancel token"
    );
}

// ============================================================================
// CONTRACT: Effect system metrics
// ============================================================================

/// CONTRACT: Effect counters are monotonic and increment by exactly 1 per call.
/// The combined total must always equal command_total + subscription_total.
#[test]
fn contract_effect_metrics_increment_monotonically() {
    let before_cmd = ftui_runtime::effect_system::effects_command_total();
    let before_sub = ftui_runtime::effect_system::effects_subscription_total();

    ftui_runtime::effect_system::record_command_effect("test_cmd", 100);
    ftui_runtime::effect_system::record_command_effect("test_cmd", 200);

    let after_cmd = ftui_runtime::effect_system::effects_command_total();
    assert_eq!(
        after_cmd - before_cmd,
        2,
        "command counter must increment by exactly 1 per call"
    );

    ftui_runtime::effect_system::record_subscription_start("test_sub", 1);

    let after_sub = ftui_runtime::effect_system::effects_subscription_total();
    assert_eq!(
        after_sub - before_sub,
        1,
        "subscription counter must increment by exactly 1 per call"
    );

    let total = ftui_runtime::effect_system::effects_executed_total();
    assert_eq!(
        total,
        after_cmd + after_sub,
        "combined total must equal cmd + sub"
    );
}

/// CONTRACT: No messages are processed after Cmd::Quit.
#[test]
fn contract_no_processing_after_quit() {
    struct CountModel {
        value: i32,
    }

    #[derive(Debug)]
    enum CMsg {
        Inc,
        Quit,
    }

    impl From<Event> for CMsg {
        fn from(_: Event) -> Self {
            CMsg::Inc
        }
    }

    impl Model for CountModel {
        type Message = CMsg;

        fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
            match msg {
                CMsg::Inc => {
                    self.value += 1;
                    Cmd::none()
                }
                CMsg::Quit => Cmd::quit(),
            }
        }

        fn view(&self, _frame: &mut Frame) {}
    }

    let mut sim = ProgramSimulator::new(CountModel { value: 0 });
    sim.init();

    sim.send(CMsg::Inc); // value = 1
    sim.send(CMsg::Quit);
    sim.send(CMsg::Inc); // must be ignored
    sim.send(CMsg::Inc); // must be ignored

    assert_eq!(sim.model().value, 1, "messages after Quit must be ignored");
    assert!(!sim.is_running());
}
