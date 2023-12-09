use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

pub struct Transition {
    pub active: bool,
    pub duration: Duration,
    pub start_time: Instant,
}

impl Transition {
    pub fn is_active(&self) -> bool {
        self.active && Instant::now() < self.start_time + self.duration
    }

    pub fn is_zero(&self) -> bool {
        self.duration.is_zero()
    }

    pub fn get_progress(&self) -> f32 {
        self.start_time
            .elapsed()
            .div_duration_f32(self.duration)
            .clamp(0.0, 1.0)
    }

    pub fn new(start_time: Instant, duration: Duration) -> Self {
        Transition {
            active: start_time <= Instant::now(),
            duration,
            start_time,
        }
    }

    pub fn duration_to_end(&self) -> Duration {
        if self.get_progress() == 1.0 {
            Duration::ZERO
        } else {
            (self.start_time + self.duration).duration_since(Instant::now())
        }
    }

    /// returns if the active status changed after the update
    pub fn update_active(&mut self) -> bool {
        let prev = self.active;
        if let Some(dur) = Instant::now().checked_duration_since(self.start_time) {
            if dur < self.duration {
                self.active = true;
                return prev != self.active; // if it just started
            }
        }
        self.active = false;
        prev != self.active //if it just ended
    }
}

pub trait StateStruct: Clone + Default + Debug {
    type StateEnum: Copy + Clone;
    fn timer_ended_callback(state_transition: &mut StateTransition<Self>);
    ///called during StateTransition initialization
    fn init_callback(state_transition: &mut StateTransition<Self>) {}
    fn get_idle_state() -> Self::StateEnum;
    fn get_state(&self) -> Self::StateEnum;
    fn set_state(&mut self, state: Self::StateEnum);
}

#[derive(Clone)]
pub struct StateTransition<T: StateStruct> {
    state: T,
    timer_ended_callback: fn(&mut Self),

    duration: Duration,
    start_time: Instant,
    enabled: bool,
    running: bool,
}

impl<T: StateStruct> Debug for StateTransition<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateTransition")
            .field("state", &self.state)
            .field("timer_ended_callback", &self.timer_ended_callback)
            .field("duration", &self.duration)
            .field("start_time", &self.start_time)
            .field("remaining-time", &self.duration_to_end())
            .field("enabled", &self.enabled)
            .field("running", &self.running)
            .finish()
    }
}

impl<T: StateStruct> Default for StateTransition<T> {
    fn default() -> Self {
        let mut sf = Self {
            state: T::default(),
            timer_ended_callback: T::timer_ended_callback,
            duration: Duration::ZERO,
            start_time: Instant::now(),
            enabled: false,
            running: false,
        };
        T::init_callback(&mut sf);
        sf
    }
}

impl<T: StateStruct> StateTransition<T> {
    pub fn get_state(&mut self) -> T::StateEnum {
        self.update_state();
        self.state.get_state()
    }
    pub fn get_state_struct(&mut self) -> &mut T {
        self.update_state();
        &mut self.state
    }

    pub fn get_progress(&self) -> f32 {
        self.start_time
            .elapsed()
            .div_duration_f32(self.duration)
            .clamp(0.0, 1.0)
    }

    pub fn duration_to_end(&self) -> Duration {
        if self.get_progress() == 1.0 {
            Duration::ZERO
        } else {
            (self.start_time + self.duration).duration_since(Instant::now())
        }
    }

    pub fn is_zero(&self) -> bool {
        self.duration.is_zero()
    }

    pub fn update_state(&mut self) {
        if self.eval_is_running() {
            return;
        }

        let was_running = self.running;
        if was_running && self.enabled {
            self.running = false;
            //timer finished
            (self.timer_ended_callback)(self);
        } else if !self.enabled {
            // should not continue after the timer has finished
            self.running = false;
            self.state.set_state(T::get_idle_state());
        }
    }
    pub fn enable(&mut self) {
        self.enabled = true;
        if !self.eval_is_running() {
            self.state.set_state(T::get_idle_state());
            (self.timer_ended_callback)(self);
        }
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// if the timer is already running it prolongs or shortens the duration,
    /// otherwise it starts it.
    /// Only works if the transition is enabled
    pub fn start_timer_duration(&mut self, duration: Duration) {
        if !self.enabled {
            return;
        }

        if self.eval_is_running() {
            self.duration = duration;
        } else {
            self.duration = duration;
            self.start_time = Instant::now();
            self.running = true;
            // (self.timer_ended_callback)(self); //change from idle, state change should be done by caller
        }
    }

    /// if the timer is already running it prolongs or shortens the duration
    /// otherwise it starts it.
    /// Only works if the transition is enabled
    ///
    /// # Examples
    /// ```
    /// timer.start_timer_time(Instant::now()); //stops the timer now
    /// ```
    ///
    pub fn start_timer_time(&mut self, end_time: Instant) {
        if !self.enabled {
            return;
        }

        let diff = end_time.duration_since(self.start_time);
        if diff == Duration::ZERO {
            return;
        }

        if self.eval_is_running() {
            self.duration = diff;
        } else {
            self.duration = end_time.duration_since(Instant::now());
            self.start_time = Instant::now();
            self.running = true;
            (self.timer_ended_callback)(self); //change from idle
        }
    }

    pub fn eval_is_running(&self) -> bool {
        let now = Instant::now();
        now >= self.start_time && now < self.start_time + self.duration
    }
}
