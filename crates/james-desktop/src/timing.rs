//! Preserve the UI approval deadline across pauses and worker dispatch.
use std::time::{Duration, Instant, SystemTime};

const COUNTDOWN: Duration = Duration::from_secs(5);
const GRACE: Duration = Duration::from_secs(2);
const SCREEN_AGE: Duration = Duration::from_secs(60);

#[derive(Clone, Copy)]
pub struct ApprovalWindow {
    pub at: Instant,
    expires_at: Instant,
    approved_wall: SystemTime,
    expires_wall: SystemTime,
    captured_at: Instant,
}
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Waiting,
    Ready,
    Expired,
}
impl ApprovalWindow {
    pub fn new(now: Instant, wall: SystemTime, captured_at: Instant) -> Self {
        Self {
            at: now + COUNTDOWN,
            expires_at: now + COUNTDOWN + GRACE,
            approved_wall: wall,
            expires_wall: wall + COUNTDOWN + GRACE,
            captured_at,
        }
    }
    pub fn state(self, now: Instant, wall: SystemTime) -> State {
        // Wall time also catches suspend on clocks that do not count sleep.
        // A backward wall-clock adjustment requires review again as well.
        if now > self.expires_at
            || wall > self.expires_wall
            || wall < self.approved_wall
            || now.saturating_duration_since(self.captured_at) > SCREEN_AGE
        {
            State::Expired
        } else if now >= self.at {
            State::Ready
        } else {
            State::Waiting
        }
    }
    pub fn current(self) -> bool {
        self.state(Instant::now(), SystemTime::now()) == State::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn countdown_has_a_bounded_dispatch_window() {
        let now = Instant::now();
        let wall = SystemTime::now();
        let window = ApprovalWindow::new(now, wall, now);
        assert_eq!(window.state(now, wall), State::Waiting);
        assert_eq!(
            window.state(now + COUNTDOWN, wall + COUNTDOWN),
            State::Ready
        );
        let late = COUNTDOWN + GRACE + Duration::from_millis(1);
        assert_eq!(window.state(now + late, wall + late), State::Expired);
    }
    #[test]
    fn sleep_and_clock_changes_do_not_reuse_approval() {
        let now = Instant::now();
        let wall = SystemTime::now();
        let window = ApprovalWindow::new(now, wall, now);
        assert_eq!(
            window.state(now + COUNTDOWN, wall + Duration::from_secs(3600)),
            State::Expired
        );
        assert_eq!(
            window.state(now + COUNTDOWN, wall - Duration::from_secs(1)),
            State::Expired
        );
    }
    #[test]
    fn capture_expiring_during_countdown_requires_review_again() {
        let now = Instant::now();
        let wall = SystemTime::now();
        let window = ApprovalWindow::new(now, wall, now - Duration::from_secs(58));
        assert_eq!(
            window.state(now + COUNTDOWN, wall + COUNTDOWN),
            State::Expired
        );
    }
}
