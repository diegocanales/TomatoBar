use crate::config::{Config, StartWith, StopAfter, MAX_INTERVAL_SECS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Work,
    ShortRest,
    LongRest,
    /// Rest finished. Next work starts when the user is active again.
    AwaitingUser,
}

impl Phase {
    pub fn is_running(self) -> bool {
        !matches!(self, Phase::Idle)
    }

    pub fn label(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Work => "work",
            Phase::ShortRest => "short_rest",
            Phase::LongRest => "long_rest",
            Phase::AwaitingUser => "awaiting_user",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerEvent {
    EnteredWork,
    EnteredShortRest,
    EnteredLongRest,
    EnteredAwaitingUser,
    EnteredIdle,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct Timer {
    pub phase: Phase,
    pub remaining_secs: u32,
    pub paused: bool,
    pub current_work: u32,
    pub config: Config,
}

impl Timer {
    pub fn new(config: Config) -> Self {
        Self {
            phase: Phase::Idle,
            remaining_secs: 0,
            paused: false,
            current_work: 0,
            config,
        }
    }

    pub fn reload_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn format_mm_ss(secs: u32) -> String {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }

    pub fn display_time(&self) -> String {
        Self::format_mm_ss(self.remaining_secs)
    }

    pub fn work_progress_label(&self) -> Option<String> {
        let n = self.config.preset().works_in_set;
        if n <= 1 || self.current_work == 0 {
            return None;
        }
        Some(format!("{}/{}", self.current_work, n))
    }

    pub fn start_stop(&mut self) -> Vec<TimerEvent> {
        if self.phase.is_running() {
            self.stop()
        } else {
            self.start()
        }
    }

    pub fn start(&mut self) -> Vec<TimerEvent> {
        self.paused = false;
        match self.config.start_with {
            StartWith::Work => self.enter_work(),
            StartWith::Rest => self.enter_short_rest_without_increment(),
        }
    }

    pub fn stop(&mut self) -> Vec<TimerEvent> {
        self.phase = Phase::Idle;
        self.remaining_secs = 0;
        self.paused = false;
        self.current_work = 0;
        vec![TimerEvent::EnteredIdle]
    }

    pub fn pause_resume(&mut self) {
        if !self.phase.is_running() || self.phase == Phase::AwaitingUser {
            return;
        }
        self.paused = !self.paused;
    }

    pub fn skip(&mut self) -> Vec<TimerEvent> {
        if !self.phase.is_running() {
            return vec![];
        }
        self.paused = false;
        let mut events = self.advance_phase();
        events.insert(0, TimerEvent::Skipped);
        events
    }

    pub fn add_minute(&mut self) {
        if !self.phase.is_running() || self.phase == Phase::AwaitingUser {
            return;
        }
        self.remaining_secs = (self.remaining_secs + 60).min(MAX_INTERVAL_SECS);
    }

    /// Reset the current interval to its full duration (work or rest).
    pub fn restart_current(&mut self) {
        if !self.phase.is_running() || self.phase == Phase::AwaitingUser {
            return;
        }
        let preset = self.config.preset();
        self.remaining_secs = match self.phase {
            Phase::Idle => 0,
            Phase::Work => preset.work_secs(),
            Phase::ShortRest => preset.short_rest_secs(),
            Phase::LongRest => preset.long_rest_secs(),
            Phase::AwaitingUser => 0,
        };
        self.paused = false;
    }

    /// Leave `AwaitingUser` and start the next work interval.
    pub fn begin_work_if_held(&mut self) -> Vec<TimerEvent> {
        if self.phase != Phase::AwaitingUser {
            return vec![];
        }
        self.enter_work()
    }

    pub fn tick(&mut self) -> Vec<TimerEvent> {
        if self.phase == Phase::AwaitingUser {
            return vec![];
        }
        if !self.phase.is_running() || self.paused {
            return vec![];
        }
        if self.remaining_secs == 0 {
            return self.advance_phase();
        }
        self.remaining_secs -= 1;
        if self.remaining_secs == 0 {
            return self.advance_phase();
        }
        vec![]
    }

    fn enter_work(&mut self) -> Vec<TimerEvent> {
        let preset = self.config.preset().clone();
        if self.current_work >= preset.works_in_set {
            self.current_work = 1;
        } else {
            self.current_work += 1;
        }
        self.phase = Phase::Work;
        self.remaining_secs = preset.work_secs();
        self.paused = false;
        vec![TimerEvent::EnteredWork]
    }

    fn enter_short_rest_without_increment(&mut self) -> Vec<TimerEvent> {
        // "start with rest" before any work completed
        self.current_work = 0;
        self.phase = Phase::ShortRest;
        self.remaining_secs = self.config.preset().short_rest_secs();
        self.paused = false;
        vec![TimerEvent::EnteredShortRest]
    }

    fn enter_short_rest(&mut self) -> Vec<TimerEvent> {
        self.phase = Phase::ShortRest;
        self.remaining_secs = self.config.preset().short_rest_secs();
        self.paused = false;
        vec![TimerEvent::EnteredShortRest]
    }

    fn enter_long_rest(&mut self) -> Vec<TimerEvent> {
        self.phase = Phase::LongRest;
        self.remaining_secs = self.config.preset().long_rest_secs();
        self.paused = false;
        vec![TimerEvent::EnteredLongRest]
    }

    fn enter_awaiting_user(&mut self) -> Vec<TimerEvent> {
        self.phase = Phase::AwaitingUser;
        self.remaining_secs = 0;
        self.paused = false;
        vec![TimerEvent::EnteredAwaitingUser]
    }

    fn after_rest(&mut self) -> Vec<TimerEvent> {
        if self.config.hold_work_until_active {
            self.enter_awaiting_user()
        } else {
            self.enter_work()
        }
    }

    fn advance_phase(&mut self) -> Vec<TimerEvent> {
        match self.phase {
            Phase::Idle => vec![],
            Phase::Work => {
                if self.config.stop_after == StopAfter::Work {
                    return self.stop();
                }
                let preset = self.config.preset().clone();
                if self.current_work >= preset.works_in_set {
                    if self.config.stop_after == StopAfter::LongRest {
                        // still show long rest, then idle after it. Handled when leaving long rest.
                    }
                    self.enter_long_rest()
                } else {
                    if self.config.stop_after == StopAfter::Rest {
                        // short rest then idle
                    }
                    self.enter_short_rest()
                }
            }
            Phase::ShortRest => {
                if self.config.stop_after == StopAfter::Rest {
                    return self.stop();
                }
                self.after_rest()
            }
            Phase::LongRest => {
                if matches!(
                    self.config.stop_after,
                    StopAfter::LongRest | StopAfter::Rest
                ) {
                    return self.stop();
                }
                self.after_rest()
            }
            Phase::AwaitingUser => self.enter_work(),
        }
    }
}
