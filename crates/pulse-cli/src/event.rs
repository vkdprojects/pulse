use crossterm::event::{Event as CtEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

pub enum Event {
    Key(KeyEvent),
    Tick,
    DataRefresh,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration, refresh_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let tx_key = tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            loop {
                if let Some(Ok(CtEvent::Key(key))) = reader.next().await
                    && tx_key.send(Event::Key(key)).is_err()
                {
                    break;
                }
            }
        });

        let tx_tick = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tx_tick.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_rate);
            loop {
                interval.tick().await;
                if tx.send(Event::DataRefresh).is_err() {
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
