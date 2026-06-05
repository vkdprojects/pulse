use crossterm::event::{Event as CtEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

pub enum Event {
    Key(KeyEvent),
    Tick,
    Refresh,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_ms: u64, refresh_secs: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let tx_key = tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(evt)) = reader.next().await {
                if let CtEvent::Key(key) = evt
                    && tx_key.send(Event::Key(key)).is_err()
                {
                    break;
                }
            }
        });

        let tx_tick = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(tick_ms));
            loop {
                interval.tick().await;
                if tx_tick.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(refresh_secs));
            loop {
                interval.tick().await;
                if tx.send(Event::Refresh).is_err() {
                    break;
                }
            }
        });

        Self { rx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

pub fn is_quit(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('q') || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}
