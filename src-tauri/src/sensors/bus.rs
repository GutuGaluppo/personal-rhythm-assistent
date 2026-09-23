use super::event::ActivityEvent;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

/// In-process fan-out of neutral events. Consumers (Sessionizer, UI) subscribe;
/// a subscriber that stops listening is dropped on the next publish.
#[derive(Default)]
pub struct EventBus {
    subscribers: Mutex<Vec<Sender<ActivityEvent>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> Receiver<ActivityEvent> {
        let (tx, rx) = channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    pub fn publish(&self, event: &ActivityEvent) {
        self.subscribers
            .lock()
            .unwrap()
            .retain(|tx| tx.send(event.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().unwrap().len()
    }
}
