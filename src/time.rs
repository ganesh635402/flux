// Flux temporal model — clock, sleeper, Instant, Duration, Date, Time, DateTime, Task.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::time;

/// A Flux instant — a point in time on the Flux clock.
/// Internally stored as nanoseconds since an epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxInstant {
    /// Nanoseconds since epoch (monotonic clock base).
    pub nanos: i128,
}

impl FluxInstant {
    pub fn from_nanos(nanos: i128) -> Self {
        FluxInstant { nanos }
    }
}

impl fmt::Display for FluxInstant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show as seconds with fractional part for readability
        let secs = self.nanos / 1_000_000_000;
        let frac = (self.nanos % 1_000_000_000).unsigned_abs();
        if frac == 0 {
            write!(f, "<instant {}s>", secs)
        } else {
            write!(f, "<instant {}.{:09}s>", secs, frac)
        }
    }
}

/// A Flux duration — an amount of elapsed time.
/// Internally stored as nanoseconds (signed, to support negative durations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxDuration {
    /// Duration in nanoseconds (can be negative).
    pub nanos: i128,
}

impl FluxDuration {
    pub fn from_nanos(nanos: i128) -> Self {
        FluxDuration { nanos }
    }

    pub fn from_micros(micros: i64) -> Self {
        FluxDuration {
            nanos: micros as i128 * 1_000,
        }
    }

    pub fn from_millis(millis: i64) -> Self {
        FluxDuration {
            nanos: millis as i128 * 1_000_000,
        }
    }

    pub fn from_secs(secs: i64) -> Self {
        FluxDuration {
            nanos: secs as i128 * 1_000_000_000,
        }
    }

    pub fn from_mins(mins: i64) -> Self {
        FluxDuration {
            nanos: mins as i128 * 60 * 1_000_000_000,
        }
    }

    pub fn from_hours(hours: i64) -> Self {
        FluxDuration {
            nanos: hours as i128 * 3600 * 1_000_000_000,
        }
    }

    pub fn from_days(days: i64) -> Self {
        FluxDuration {
            nanos: days as i128 * 86400 * 1_000_000_000,
        }
    }

    /// Convert to std::time::Duration for sleeping. Returns None if negative.
    pub fn to_std_duration(&self) -> Option<time::Duration> {
        if self.nanos < 0 {
            None
        } else {
            Some(time::Duration::from_nanos(self.nanos as u64))
        }
    }
}

impl fmt::Display for FluxDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.nanos.unsigned_abs();

        if abs == 0 {
            return write!(f, "0s");
        }

        // Choose the most readable unit
        if abs % 1_000_000_000 == 0 {
            let secs = self.nanos / 1_000_000_000;
            if secs.unsigned_abs() % 86400 == 0 && secs != 0 {
                write!(f, "{}d", secs / 86400)
            } else if secs.unsigned_abs() % 3600 == 0 && secs != 0 {
                write!(f, "{}h", secs / 3600)
            } else if secs.unsigned_abs() % 60 == 0 && secs != 0 {
                write!(f, "{}m", secs / 60)
            } else {
                write!(f, "{}s", secs)
            }
        } else if abs % 1_000_000 == 0 {
            write!(f, "{}ms", self.nanos / 1_000_000)
        } else if abs % 1_000 == 0 {
            write!(f, "{}us", self.nanos / 1_000)
        } else {
            write!(f, "{}ns", self.nanos)
        }
    }
}

/// Clock trait — provides the current time.
pub trait Clock: Send {
    fn now(&self) -> FluxInstant;
}

/// Sleeper trait — provides sleep functionality.
pub trait Sleeper: Send {
    fn sleep(&self, duration: &FluxDuration);
}

/// System clock using std monotonic time.
pub struct SystemClock {
    epoch: time::Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        SystemClock {
            epoch: time::Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> FluxInstant {
        let elapsed = self.epoch.elapsed();
        FluxInstant::from_nanos(elapsed.as_nanos() as i128)
    }
}

/// System sleeper — actually blocks the thread.
pub struct SystemSleeper;

impl Sleeper for SystemSleeper {
    fn sleep(&self, duration: &FluxDuration) {
        if let Some(d) = duration.to_std_duration() {
            std::thread::sleep(d);
        }
    }
}

/// Test clock with deterministic time.
#[cfg(test)]
pub struct TestClock {
    nanos: std::cell::Cell<i128>,
}

#[cfg(test)]
impl TestClock {
    pub fn new() -> Self {
        TestClock {
            nanos: std::cell::Cell::new(0),
        }
    }

    pub fn set_nanos(&self, nanos: i128) {
        self.nanos.set(nanos);
    }

    pub fn advance(&self, duration: &FluxDuration) {
        self.nanos.set(self.nanos.get() + duration.nanos);
    }
}

#[cfg(test)]
impl Clock for TestClock {
    fn now(&self) -> FluxInstant {
        FluxInstant::from_nanos(self.nanos.get())
    }
}

/// Test sleeper — records sleep calls without waiting.
#[cfg(test)]
pub struct TestSleeper {
    pub sleeps: std::cell::RefCell<Vec<FluxDuration>>,
}

#[cfg(test)]
impl TestSleeper {
    pub fn new() -> Self {
        TestSleeper {
            sleeps: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn sleep_count(&self) -> usize {
        self.sleeps.borrow().len()
    }

    pub fn last_sleep(&self) -> Option<FluxDuration> {
        self.sleeps.borrow().last().copied()
    }
}

#[cfg(test)]
impl Sleeper for TestSleeper {
    fn sleep(&self, duration: &FluxDuration) {
        self.sleeps.borrow_mut().push(*duration);
    }
}

// === Calendar types ===

/// Check whether a year is a leap year.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Return the number of days in a given month (1-based) of a given year.
pub fn days_in_month(year: i32, month: u32) -> Option<u32> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if is_leap_year(year) { 29 } else { 28 }),
        _ => None,
    }
}

/// A Flux calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FluxDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl FluxDate {
    /// Create a validated date. Returns Err message on invalid input.
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, String> {
        if !(1..=12).contains(&month) {
            return Err(format!("invalid month: {}", month));
        }
        let max_day = days_in_month(year, month).unwrap();
        if day < 1 || day > max_day {
            return Err(format!(
                "invalid day {} for {}-{:02} (max {})",
                day, year, month, max_day
            ));
        }
        Ok(FluxDate { year, month, day })
    }

    /// Convert to a day-ordinal for ordering/arithmetic (days since a reference epoch).
    /// Uses a simplified proleptic Gregorian calendar calculation.
    pub fn to_days(&self) -> i64 {
        let mut y = self.year as i64;
        let mut m = self.month as i64;
        if m <= 2 {
            y -= 1;
            m += 12;
        }
        // Days from year
        let days = 365 * y + y / 4 - y / 100 + y / 400 + (153 * (m - 3) + 2) / 5 + self.day as i64
            - 719469;
        days
    }

    /// Create a date from a day-ordinal (inverse of to_days).
    pub fn from_days(mut days: i64) -> Self {
        days += 719468;
        let era = if days >= 0 {
            days / 146097
        } else {
            (days - 146096) / 146097
        };
        let doe = (days - era * 146097) as u32;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if m <= 2 { y + 1 } else { y };
        FluxDate {
            year: year as i32,
            month: m,
            day: d,
        }
    }

    /// Return the weekday name (Monday = 0).
    pub fn weekday_name(&self) -> &'static str {
        // 2000-01-03 was a Monday. Use that as reference.
        let days = self.to_days();
        // Unix epoch (1970-01-01) was a Thursday (day 4, 0=Mon).
        let wd = ((days % 7) + 7 + 3) % 7; // +3 because 1970-01-01 = Thursday
        match wd {
            0 => "Monday",
            1 => "Tuesday",
            2 => "Wednesday",
            3 => "Thursday",
            4 => "Friday",
            5 => "Saturday",
            6 => "Sunday",
            _ => unreachable!(),
        }
    }
}

impl PartialOrd for FluxDate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FluxDate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_days().cmp(&other.to_days())
    }
}

impl fmt::Display for FluxDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// A Flux time-of-day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FluxTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub nanosecond: u32,
}

impl FluxTime {
    pub fn new(hour: u32, minute: u32, second: u32, nanosecond: u32) -> Result<Self, String> {
        if hour > 23 {
            return Err(format!("invalid hour: {}", hour));
        }
        if minute > 59 {
            return Err(format!("invalid minute: {}", minute));
        }
        if second > 59 {
            return Err(format!("invalid second: {}", second));
        }
        if nanosecond > 999_999_999 {
            return Err(format!("invalid nanosecond: {}", nanosecond));
        }
        Ok(FluxTime {
            hour,
            minute,
            second,
            nanosecond,
        })
    }

    fn to_nanos(&self) -> u64 {
        (self.hour as u64 * 3600 + self.minute as u64 * 60 + self.second as u64) * 1_000_000_000
            + self.nanosecond as u64
    }
}

impl PartialOrd for FluxTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FluxTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_nanos().cmp(&other.to_nanos())
    }
}

impl fmt::Display for FluxTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.nanosecond != 0 {
            write!(
                f,
                "{:02}:{:02}:{:02}.{:09}",
                self.hour, self.minute, self.second, self.nanosecond
            )
        } else {
            write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
        }
    }
}

/// A Flux wall-clock datetime (date + time, local timezone assumed for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FluxDateTime {
    pub date: FluxDate,
    pub time: FluxTime,
}

impl FluxDateTime {
    pub fn new(date: FluxDate, time: FluxTime) -> Self {
        FluxDateTime { date, time }
    }

    /// Convert to a total nanosecond offset from epoch for arithmetic.
    pub fn to_epoch_nanos(&self) -> i128 {
        let day_nanos = self.date.to_days() as i128 * 86_400_000_000_000i128;
        day_nanos + self.time.to_nanos() as i128
    }

    /// Create from total nanoseconds since epoch.
    pub fn from_epoch_nanos(nanos: i128) -> Self {
        let day_nanos = 86_400_000_000_000i128;
        let days = nanos.div_euclid(day_nanos);
        let mut remainder = nanos.rem_euclid(day_nanos) as u64;

        let date = FluxDate::from_days(days as i64);

        let hour = (remainder / 3_600_000_000_000) as u32;
        remainder %= 3_600_000_000_000;
        let minute = (remainder / 60_000_000_000) as u32;
        remainder %= 60_000_000_000;
        let second = (remainder / 1_000_000_000) as u32;
        let nanosecond = (remainder % 1_000_000_000) as u32;

        FluxDateTime {
            date,
            time: FluxTime {
                hour,
                minute,
                second,
                nanosecond,
            },
        }
    }
}

impl PartialOrd for FluxDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FluxDateTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_epoch_nanos().cmp(&other.to_epoch_nanos())
    }
}

impl fmt::Display for FluxDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.time.nanosecond != 0 {
            write!(f, "{} {}", self.date, self.time)
        } else {
            write!(f, "{} {}", self.date, self.time)
        }
    }
}

// === Task types ===

/// The state of a scheduled task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskState::Pending => write!(f, "pending"),
            TaskState::Running => write!(f, "running"),
            TaskState::Completed => write!(f, "completed"),
            TaskState::Failed => write!(f, "failed"),
            TaskState::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A Flux task handle — a reference to a scheduled task.
/// Thread-safe: uses Arc<Mutex<...>> so task state can be observed from any thread.
#[derive(Debug, Clone)]
pub struct FluxTask {
    pub id: u64,
    inner: Arc<Mutex<TaskInner>>,
    condvar: Arc<Condvar>,
}

#[derive(Debug)]
struct TaskInner {
    state: TaskState,
    result: Option<crate::runtime::Value>,
    error: Option<String>,
    is_recurring: bool,
}

impl FluxTask {
    pub fn new(id: u64) -> Self {
        FluxTask {
            id,
            inner: Arc::new(Mutex::new(TaskInner {
                state: TaskState::Pending,
                result: None,
                error: None,
                is_recurring: false,
            })),
            condvar: Arc::new(Condvar::new()),
        }
    }

    pub fn new_recurring(id: u64) -> Self {
        FluxTask {
            id,
            inner: Arc::new(Mutex::new(TaskInner {
                state: TaskState::Pending,
                result: None,
                error: None,
                is_recurring: true,
            })),
            condvar: Arc::new(Condvar::new()),
        }
    }

    pub fn state(&self) -> TaskState {
        self.inner.lock().unwrap().state
    }

    pub fn is_cancelled(&self) -> bool {
        self.state() == TaskState::Cancelled
    }

    pub fn is_done(&self) -> bool {
        matches!(
            self.state(),
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        )
    }

    pub fn is_running(&self) -> bool {
        self.state() == TaskState::Running
    }

    pub fn is_recurring(&self) -> bool {
        self.inner.lock().unwrap().is_recurring
    }

    pub fn set_state(&self, state: TaskState) {
        self.inner.lock().unwrap().state = state;
        if matches!(
            state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        ) {
            self.condvar.notify_all();
        }
    }

    pub fn cancel(&self) {
        self.set_state(TaskState::Cancelled);
    }

    pub fn set_result(&self, value: crate::runtime::Value) {
        let mut inner = self.inner.lock().unwrap();
        inner.result = Some(value);
    }

    pub fn set_error(&self, msg: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.error = Some(msg);
    }

    pub fn get_result(&self) -> Option<crate::runtime::Value> {
        self.inner.lock().unwrap().result.clone()
    }

    pub fn get_error(&self) -> Option<String> {
        self.inner.lock().unwrap().error.clone()
    }

    /// Wait for the task to reach a terminal state (Completed/Failed/Cancelled).
    /// Uses condvar — no busy polling.
    pub fn wait_done(&self) {
        let inner = self.inner.lock().unwrap();
        if matches!(
            inner.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        ) {
            return;
        }
        let _guard = self
            .condvar
            .wait_while(inner, |i| {
                !matches!(
                    i.state,
                    TaskState::Completed | TaskState::Failed | TaskState::Cancelled
                )
            })
            .unwrap();
    }
}

impl fmt::Display for FluxTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<task {} {}>", self.id, self.state())
    }
}

// === Calendar recurrence ===

/// A calendar-based recurrence pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum CalendarRecurrence {
    /// Every calendar day
    Daily,
    /// Every specified weekday (0=Monday..6=Sunday)
    Weekly(u32),
    /// Every month on the specified day
    Monthly(u32),
    /// Every year on the specified month and day
    Yearly(u32, u32),
}

impl CalendarRecurrence {
    /// Calculate the next occurrence DateTime after `current`.
    /// If `current` is before today's target time, returns today's target.
    /// Otherwise returns the next valid calendar occurrence.
    pub fn next_occurrence(&self, current: &FluxDateTime, target_time: &FluxTime) -> FluxDateTime {
        let today_target = FluxDateTime::new(current.date, *target_time);

        // Check if today's target is still in the future and matches
        if today_target > *current && self.matches_date(&current.date) {
            return today_target;
        }

        // Find next matching date after today
        let mut candidate_days = current.date.to_days() + 1;
        for _ in 0..400 {
            let candidate = FluxDate::from_days(candidate_days);
            if self.matches_date(&candidate) {
                return FluxDateTime::new(candidate, *target_time);
            }
            candidate_days += 1;
        }

        // Fallback
        FluxDateTime::new(FluxDate::from_days(candidate_days), *target_time)
    }

    /// Check if a date matches this recurrence pattern.
    fn matches_date(&self, date: &FluxDate) -> bool {
        match self {
            CalendarRecurrence::Daily => true,
            CalendarRecurrence::Weekly(weekday) => {
                let wd = ((date.to_days() % 7) + 7 + 3) % 7;
                wd as u32 == *weekday
            }
            CalendarRecurrence::Monthly(day) => {
                let max_day = days_in_month(date.year, date.month).unwrap_or(28);
                let effective_day = (*day).min(max_day);
                date.day == effective_day
            }
            CalendarRecurrence::Yearly(month, day) => {
                if date.month != *month {
                    return false;
                }
                let max_day = days_in_month(date.year, *month).unwrap_or(28);
                let effective_day = (*day).min(max_day);
                date.day == effective_day
            }
        }
    }

    /// Convert a weekday name to a number (0=Monday..6=Sunday).
    pub fn weekday_number(name: &str) -> Option<u32> {
        match name {
            "Monday" => Some(0),
            "Tuesday" => Some(1),
            "Wednesday" => Some(2),
            "Thursday" => Some(3),
            "Friday" => Some(4),
            "Saturday" => Some(5),
            "Sunday" => Some(6),
            _ => None,
        }
    }
}

/// Wall clock trait — provides the current calendar datetime.
pub trait WallClock: Send {
    fn datetime(&self) -> FluxDateTime;
}

/// System wall clock — uses local system time.
pub struct SystemWallClock;

impl WallClock for SystemWallClock {
    fn datetime(&self) -> FluxDateTime {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let total_nanos = now.as_nanos() as i128;
        // Convert UTC epoch nanos to local time by adding local offset
        // For now, use UTC. Timezone support deferred to a future stage.
        FluxDateTime::from_epoch_nanos(total_nanos)
    }
}

/// Test wall clock — deterministic, no real system time.
#[cfg(test)]
pub struct TestWallClock {
    dt: std::cell::RefCell<FluxDateTime>,
}

#[cfg(test)]
impl TestWallClock {
    pub fn new(dt: FluxDateTime) -> Self {
        TestWallClock {
            dt: std::cell::RefCell::new(dt),
        }
    }

    pub fn set(&self, dt: FluxDateTime) {
        *self.dt.borrow_mut() = dt;
    }
}

#[cfg(test)]
impl WallClock for TestWallClock {
    fn datetime(&self) -> FluxDateTime {
        *self.dt.borrow()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_from_secs() {
        let d = FluxDuration::from_secs(5);
        assert_eq!(d.nanos, 5_000_000_000);
    }

    #[test]
    fn duration_from_millis() {
        let d = FluxDuration::from_millis(500);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn duration_from_mins() {
        let d = FluxDuration::from_mins(2);
        assert_eq!(d.nanos, 120_000_000_000);
    }

    #[test]
    fn duration_from_hours() {
        let d = FluxDuration::from_hours(1);
        assert_eq!(d.nanos, 3_600_000_000_000);
    }

    #[test]
    fn duration_from_days() {
        let d = FluxDuration::from_days(1);
        assert_eq!(d.nanos, 86_400_000_000_000);
    }

    #[test]
    fn duration_display_seconds() {
        assert_eq!(format!("{}", FluxDuration::from_secs(5)), "5s");
        assert_eq!(format!("{}", FluxDuration::from_secs(0)), "0s");
    }

    #[test]
    fn duration_display_millis() {
        assert_eq!(format!("{}", FluxDuration::from_millis(500)), "500ms");
    }

    #[test]
    fn duration_display_minutes() {
        assert_eq!(format!("{}", FluxDuration::from_mins(2)), "2m");
    }

    #[test]
    fn duration_display_hours() {
        assert_eq!(format!("{}", FluxDuration::from_hours(1)), "1h");
    }

    #[test]
    fn duration_display_days() {
        assert_eq!(format!("{}", FluxDuration::from_days(1)), "1d");
    }

    #[test]
    fn duration_display_negative() {
        assert_eq!(format!("{}", FluxDuration::from_secs(-5)), "-5s");
    }

    #[test]
    fn duration_equality() {
        assert_eq!(FluxDuration::from_secs(1), FluxDuration::from_millis(1000));
        assert_eq!(FluxDuration::from_mins(1), FluxDuration::from_secs(60));
    }

    #[test]
    fn duration_ordering() {
        assert!(FluxDuration::from_secs(5) > FluxDuration::from_secs(2));
        assert!(FluxDuration::from_secs(1) < FluxDuration::from_secs(10));
    }

    #[test]
    fn duration_arithmetic() {
        let a = FluxDuration::from_secs(10);
        let b = FluxDuration::from_secs(5);
        assert_eq!(
            FluxDuration::from_nanos(a.nanos + b.nanos),
            FluxDuration::from_secs(15)
        );
        assert_eq!(
            FluxDuration::from_nanos(a.nanos - b.nanos),
            FluxDuration::from_secs(5)
        );
    }

    #[test]
    fn duration_to_std() {
        let d = FluxDuration::from_millis(100);
        let std_d = d.to_std_duration().unwrap();
        assert_eq!(std_d.as_millis(), 100);
    }

    #[test]
    fn duration_negative_to_std() {
        let d = FluxDuration::from_secs(-5);
        assert!(d.to_std_duration().is_none());
    }

    #[test]
    fn instant_ordering() {
        let a = FluxInstant::from_nanos(100);
        let b = FluxInstant::from_nanos(200);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, a);
    }

    #[test]
    fn test_clock_deterministic() {
        let clock = TestClock::new();
        assert_eq!(clock.now(), FluxInstant::from_nanos(0));
        clock.set_nanos(1_000_000_000);
        assert_eq!(clock.now(), FluxInstant::from_nanos(1_000_000_000));
    }

    #[test]
    fn test_clock_advance() {
        let clock = TestClock::new();
        clock.advance(&FluxDuration::from_secs(5));
        assert_eq!(clock.now(), FluxInstant::from_nanos(5_000_000_000));
        clock.advance(&FluxDuration::from_millis(500));
        assert_eq!(clock.now(), FluxInstant::from_nanos(5_500_000_000));
    }

    #[test]
    fn test_sleeper_records() {
        let sleeper = TestSleeper::new();
        sleeper.sleep(&FluxDuration::from_secs(5));
        sleeper.sleep(&FluxDuration::from_millis(100));
        assert_eq!(sleeper.sleep_count(), 2);
        assert_eq!(sleeper.last_sleep(), Some(FluxDuration::from_millis(100)));
    }

    #[test]
    fn system_clock_monotonic() {
        let clock = SystemClock::new();
        let a = clock.now();
        let b = clock.now();
        assert!(b.nanos >= a.nanos);
    }

    // === Calendar tests ===

    #[test]
    fn date_valid() {
        assert!(FluxDate::new(2026, 8, 30).is_ok());
        assert!(FluxDate::new(2024, 2, 29).is_ok()); // leap year
    }

    #[test]
    fn date_invalid_month() {
        assert!(FluxDate::new(2026, 0, 1).is_err());
        assert!(FluxDate::new(2026, 13, 1).is_err());
    }

    #[test]
    fn date_invalid_day() {
        assert!(FluxDate::new(2026, 2, 30).is_err());
        assert!(FluxDate::new(2025, 2, 29).is_err()); // not a leap year
        assert!(FluxDate::new(2026, 4, 31).is_err());
    }

    #[test]
    fn date_display() {
        let d = FluxDate::new(2026, 8, 30).unwrap();
        assert_eq!(format!("{}", d), "2026-08-30");
    }

    #[test]
    fn date_ordering() {
        let a = FluxDate::new(2026, 8, 29).unwrap();
        let b = FluxDate::new(2026, 8, 30).unwrap();
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn date_roundtrip() {
        let d = FluxDate::new(2026, 8, 30).unwrap();
        let days = d.to_days();
        let d2 = FluxDate::from_days(days);
        assert_eq!(d, d2);
    }

    #[test]
    fn date_arithmetic_add_day() {
        let d = FluxDate::new(2026, 8, 30).unwrap();
        let d2 = FluxDate::from_days(d.to_days() + 1);
        assert_eq!(d2, FluxDate::new(2026, 8, 31).unwrap());
    }

    #[test]
    fn date_arithmetic_month_boundary() {
        let d = FluxDate::new(2026, 8, 31).unwrap();
        let d2 = FluxDate::from_days(d.to_days() + 1);
        assert_eq!(d2, FluxDate::new(2026, 9, 1).unwrap());
    }

    #[test]
    fn date_weekday() {
        // 2026-08-30 is a Sunday
        let d = FluxDate::new(2026, 8, 30).unwrap();
        assert_eq!(d.weekday_name(), "Sunday");
        // 2024-01-01 was a Monday
        let d = FluxDate::new(2024, 1, 1).unwrap();
        assert_eq!(d.weekday_name(), "Monday");
    }

    #[test]
    fn leap_year_check() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
    }

    #[test]
    fn days_in_month_check() {
        assert_eq!(days_in_month(2026, 2), Some(28));
        assert_eq!(days_in_month(2024, 2), Some(29));
        assert_eq!(days_in_month(2026, 1), Some(31));
        assert_eq!(days_in_month(2026, 4), Some(30));
        assert_eq!(days_in_month(2026, 13), None);
    }

    #[test]
    fn time_valid() {
        assert!(FluxTime::new(0, 0, 0, 0).is_ok());
        assert!(FluxTime::new(23, 59, 59, 999_999_999).is_ok());
    }

    #[test]
    fn time_invalid() {
        assert!(FluxTime::new(24, 0, 0, 0).is_err());
        assert!(FluxTime::new(0, 60, 0, 0).is_err());
        assert!(FluxTime::new(0, 0, 60, 0).is_err());
    }

    #[test]
    fn time_display() {
        let t = FluxTime::new(14, 30, 15, 0).unwrap();
        assert_eq!(format!("{}", t), "14:30:15");
    }

    #[test]
    fn time_ordering() {
        let a = FluxTime::new(10, 0, 0, 0).unwrap();
        let b = FluxTime::new(18, 0, 0, 0).unwrap();
        assert!(a < b);
    }

    #[test]
    fn datetime_display() {
        let dt = FluxDateTime::new(
            FluxDate::new(2026, 8, 30).unwrap(),
            FluxTime::new(14, 30, 15, 0).unwrap(),
        );
        assert_eq!(format!("{}", dt), "2026-08-30 14:30:15");
    }

    #[test]
    fn datetime_ordering() {
        let a = FluxDateTime::new(
            FluxDate::new(2026, 8, 30).unwrap(),
            FluxTime::new(10, 0, 0, 0).unwrap(),
        );
        let b = FluxDateTime::new(
            FluxDate::new(2026, 8, 30).unwrap(),
            FluxTime::new(11, 0, 0, 0).unwrap(),
        );
        assert!(a < b);
    }

    #[test]
    fn datetime_epoch_roundtrip() {
        let dt = FluxDateTime::new(
            FluxDate::new(2026, 8, 30).unwrap(),
            FluxTime::new(14, 30, 15, 0).unwrap(),
        );
        let nanos = dt.to_epoch_nanos();
        let dt2 = FluxDateTime::from_epoch_nanos(nanos);
        assert_eq!(dt, dt2);
    }

    #[test]
    fn datetime_add_duration() {
        let dt = FluxDateTime::new(
            FluxDate::new(2026, 8, 30).unwrap(),
            FluxTime::new(14, 30, 0, 0).unwrap(),
        );
        let nanos = dt.to_epoch_nanos() + FluxDuration::from_hours(2).nanos;
        let result = FluxDateTime::from_epoch_nanos(nanos);
        assert_eq!(result.time.hour, 16);
        assert_eq!(result.time.minute, 30);
    }

    #[test]
    fn test_wall_clock_deterministic() {
        let dt = FluxDateTime::new(
            FluxDate::new(2026, 8, 30).unwrap(),
            FluxTime::new(10, 0, 0, 0).unwrap(),
        );
        let wc = TestWallClock::new(dt);
        assert_eq!(wc.datetime(), dt);
    }
}
