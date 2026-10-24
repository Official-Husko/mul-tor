//! Feeding the interface: keyboard and paste events from the terminal, and a
//! clock.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle as TaskHandle;
use tokio::time::MissedTickBehavior;

use super::msg::Msg;

/// How often the clock ticks (animation, notices fading).
pub const TICK: Duration = Duration::from_millis(100);

/// How long the input thread waits for an event before checking whether it
/// should stop.
const POLL: Duration = Duration::from_millis(100);

/// A thread that forwards terminal events as [`Msg`]s.
///
/// It is a thread, not a task, because reading terminal input blocks.
pub struct Input {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Input {
    pub fn spawn(tx: UnboundedSender<Msg>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("mul-tor-input".into())
            .spawn(move || forward(&flag, &tx))
            .ok();
        Self { stop, thread }
    }

    /// Stop reading and wait for the thread, so it is gone before the terminal
    /// is handed back to the shell.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn forward(stop: &AtomicBool, tx: &UnboundedSender<Msg>) {
    while !stop.load(Ordering::Relaxed) {
        match event::poll(POLL) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_) => return,
        }
        let msg = match event::read() {
            // Windows also reports key releases; only presses and repeats count.
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => Msg::Key(key),
            Ok(Event::Paste(text)) => Msg::Paste(text),
            // The next draw picks up the new size; a tick makes sure there is one.
            Ok(Event::Resize(..)) => Msg::Tick,
            Ok(_) => continue,
            Err(_) => return,
        };
        if tx.send(msg).is_err() {
            return;
        }
    }
}

/// Send a [`Msg::Tick`] every [`TICK`] until the receiver is gone.
pub fn spawn_clock(tx: UnboundedSender<Msg>) -> TaskHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TICK);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if tx.send(Msg::Tick).is_err() {
                return;
            }
        }
    })
}
