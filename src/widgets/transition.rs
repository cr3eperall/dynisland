use std::time::{Duration, Instant};

pub struct Transition {
    pub active: bool,
    pub duration: Duration,
    pub start_time: Instant,
}

impl Transition {
    pub fn is_active(&self) -> bool {
        self.active && Instant::now() < self.start_time + self.duration
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

    pub fn update_active(&mut self) {
        if let Some(dur) = Instant::now().checked_duration_since(self.start_time) {
            if dur < self.duration {
                self.active = true;
                return;
            }
        }
        self.active = false;
    }
}
