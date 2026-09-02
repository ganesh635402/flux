// Flux scheduler — manages scheduled tasks for `after` and `every`.

use crate::ast::Block;
use crate::runtime::Environment;
use crate::time::{CalendarRecurrence, FluxDuration, FluxInstant, FluxTask, FluxTime, TaskState};

/// A scheduled task.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// Monotonically increasing ID for FIFO ordering of same-time tasks.
    pub id: u64,
    /// When this task should next execute.
    pub next_run: FluxInstant,
    /// Recurring interval for duration-based tasks (None for one-shot/calendar).
    pub interval: Option<FluxDuration>,
    /// Calendar recurrence pattern and target time (for calendar-based tasks).
    pub calendar: Option<(CalendarRecurrence, FluxTime)>,
    /// The block to execute.
    pub body: Block,
    /// The captured environment at scheduling time.
    pub env: Environment,
    /// The full task handle for result storage and state.
    pub task_handle: FluxTask,
}

/// The Flux scheduler — holds pending scheduled tasks.
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
    next_id: u64,
}

impl Scheduler {
    /// Create a new empty scheduler.
    pub fn new() -> Self {
        Scheduler {
            tasks: Vec::new(),
            next_id: 0,
        }
    }

    /// Get the next unique task ID (for spawn which creates tasks outside the scheduler).
    pub fn next_task_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Schedule a one-shot task (`after` / `at`). Returns a FluxTask handle.
    pub fn add_after(&mut self, run_at: FluxInstant, body: Block, env: Environment) -> FluxTask {
        let id = self.next_id;
        self.next_id += 1;
        let task_handle = FluxTask::new(id);
        self.tasks.push(ScheduledTask {
            id,
            next_run: run_at,
            interval: None,
            calendar: None,
            body,
            env,
            task_handle: task_handle.clone(),
        });
        task_handle
    }

    /// Schedule a recurring duration-based task (`every`). Returns a FluxTask handle.
    pub fn add_every(
        &mut self,
        first_run: FluxInstant,
        interval: FluxDuration,
        body: Block,
        env: Environment,
    ) -> FluxTask {
        let id = self.next_id;
        self.next_id += 1;
        let task_handle = FluxTask::new_recurring(id);
        self.tasks.push(ScheduledTask {
            id,
            next_run: first_run,
            interval: Some(interval),
            calendar: None,
            body,
            env,
            task_handle: task_handle.clone(),
        });
        task_handle
    }

    /// Schedule a recurring calendar-based task. Returns a FluxTask handle.
    pub fn add_calendar(
        &mut self,
        first_run: FluxInstant,
        recurrence: CalendarRecurrence,
        target_time: FluxTime,
        body: Block,
        env: Environment,
    ) -> FluxTask {
        let id = self.next_id;
        self.next_id += 1;
        let task_handle = FluxTask::new_recurring(id);
        self.tasks.push(ScheduledTask {
            id,
            next_run: first_run,
            interval: None,
            calendar: Some((recurrence, target_time)),
            body,
            env,
            task_handle: task_handle.clone(),
        });
        task_handle
    }

    /// Take all non-cancelled tasks whose next_run <= now, sorted by (next_run, id).
    pub fn take_due(&mut self, now: FluxInstant) -> Vec<ScheduledTask> {
        let mut due = Vec::new();
        let mut pending = Vec::new();
        for task in self.tasks.drain(..) {
            if task.task_handle.state() == TaskState::Cancelled {
                continue; // Drop cancelled tasks
            }
            if task.next_run <= now {
                due.push(task);
            } else {
                pending.push(task);
            }
        }
        self.tasks = pending;
        due.sort_by(|a, b| a.next_run.cmp(&b.next_run).then(a.id.cmp(&b.id)));
        due
    }

    /// Re-add a recurring task with updated next_run time (if not cancelled).
    pub fn reschedule(&mut self, mut task: ScheduledTask, now: FluxInstant) {
        if task.task_handle.state() == TaskState::Cancelled {
            return;
        }
        if let Some(interval) = task.interval {
            task.next_run = FluxInstant::from_nanos(now.nanos + interval.nanos);
            task.task_handle.set_state(TaskState::Pending);
            self.tasks.push(task);
        }
    }

    /// Re-add a calendar recurring task with a specific next_run time.
    pub fn reschedule_at(&mut self, mut task: ScheduledTask, next_run: FluxInstant) {
        if task.task_handle.state() == TaskState::Cancelled {
            return;
        }
        task.next_run = next_run;
        task.task_handle.set_state(TaskState::Pending);
        self.tasks.push(task);
    }

    /// Check if a task is recurring (duration or calendar).
    pub fn is_recurring(task: &ScheduledTask) -> bool {
        task.interval.is_some() || task.calendar.is_some()
    }

    /// Whether any non-cancelled tasks are pending.
    pub fn has_tasks(&self) -> bool {
        self.tasks
            .iter()
            .any(|t| t.task_handle.state() != TaskState::Cancelled)
    }

    /// Get the earliest next_run time among pending tasks.
    pub fn next_run_time(&self) -> Option<FluxInstant> {
        self.tasks.iter().map(|t| t.next_run).min()
    }

    /// Clear all scheduled tasks.
    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    /// Number of pending tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Block;
    use crate::lexer::Span;
    use crate::time::FluxDuration;

    fn empty_block() -> Block {
        Block {
            statements: Vec::new(),
            span: Span { line: 1, column: 1 },
        }
    }

    #[test]
    fn scheduler_add_after() {
        let mut sched = Scheduler::new();
        let task = sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        assert_eq!(task.id, 0);
        assert!(sched.has_tasks());
        assert_eq!(sched.task_count(), 1);
    }

    #[test]
    fn scheduler_add_every() {
        let mut sched = Scheduler::new();
        let _task = sched.add_every(
            FluxInstant::from_nanos(5_000_000_000),
            FluxDuration::from_secs(5),
            empty_block(),
            Environment::new(),
        );
        assert!(sched.has_tasks());
    }

    #[test]
    fn scheduler_take_due_none() {
        let mut sched = Scheduler::new();
        sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        let due = sched.take_due(FluxInstant::from_nanos(4_000_000_000));
        assert!(due.is_empty());
        assert!(sched.has_tasks()); // still pending
    }

    #[test]
    fn scheduler_take_due_ready() {
        let mut sched = Scheduler::new();
        sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        let due = sched.take_due(FluxInstant::from_nanos(5_000_000_000));
        assert_eq!(due.len(), 1);
        assert!(!sched.has_tasks()); // one-shot removed
    }

    #[test]
    fn scheduler_fifo_ordering() {
        let mut sched = Scheduler::new();
        let task_a = sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        let task_b = sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        let due = sched.take_due(FluxInstant::from_nanos(5_000_000_000));
        assert_eq!(due.len(), 2);
        assert_eq!(due[0].id, task_a.id);
        assert_eq!(due[1].id, task_b.id);
    }

    #[test]
    fn scheduler_next_run_time() {
        let mut sched = Scheduler::new();
        sched.add_after(
            FluxInstant::from_nanos(10_000_000_000),
            empty_block(),
            Environment::new(),
        );
        sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        assert_eq!(
            sched.next_run_time(),
            Some(FluxInstant::from_nanos(5_000_000_000))
        );
    }

    #[test]
    fn scheduler_reschedule() {
        let mut sched = Scheduler::new();
        let _task = sched.add_every(
            FluxInstant::from_nanos(5_000_000_000),
            FluxDuration::from_secs(5),
            empty_block(),
            Environment::new(),
        );
        let due = sched.take_due(FluxInstant::from_nanos(5_000_000_000));
        assert_eq!(due.len(), 1);
        assert!(!sched.has_tasks());

        // Reschedule
        let now = FluxInstant::from_nanos(5_000_000_000);
        sched.reschedule(due.into_iter().next().unwrap(), now);
        assert!(sched.has_tasks());
        assert_eq!(
            sched.next_run_time(),
            Some(FluxInstant::from_nanos(10_000_000_000))
        );
    }

    #[test]
    fn scheduler_clear() {
        let mut sched = Scheduler::new();
        sched.add_after(
            FluxInstant::from_nanos(5_000_000_000),
            empty_block(),
            Environment::new(),
        );
        sched.clear();
        assert!(!sched.has_tasks());
    }
}
