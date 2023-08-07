use std::marker::PhantomData;

use tokio::sync::broadcast;

pub struct EventBus<E> {
    tx: broadcast::Sender<E>,
}

impl<E: Clone> EventBus<E> {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn get_receiver<T: TryFrom<E>>(&self) -> EventReceiver<E, T> {
        EventReceiver {
            rx: self.tx.subscribe(),
            phantom: PhantomData,
        }
    }

    pub fn get_sender<T: Into<E>>(&self) -> EventSender<E, T> {
        EventSender {
            tx: self.tx.clone(),
            phantom: PhantomData,
        }
    }
}

pub struct EventSender<E, T> {
    tx: broadcast::Sender<E>,
    phantom: PhantomData<fn(T)>,
}

impl<E: From<T>, T> EventSender<E, T> {
    pub fn send(&self, value: T) -> Result<(), broadcast::error::SendError<E>> {
        self.tx.send(value.into()).map(drop)
    }
}

pub struct EventReceiver<E, T> {
    rx: broadcast::Receiver<E>,
    phantom: PhantomData<fn() -> T>,
}

impl<E: Clone, T: TryFrom<E>> EventReceiver<E, T> {
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        loop {
            let general_event = self.rx.recv().await?;
            if let Ok(specific_event) = general_event.try_into() {
                return Ok(specific_event);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Event1 {
        pub id: u32,
    }

    #[derive(Debug, Clone)]
    pub enum Events {
        Event1(Event1),
    }

    impl TryFrom<Events> for Event1 {
        type Error = Events;

        fn try_from(value: Events) -> Result<Self, Self::Error> {
            match value {
                Events::Event1(event) => Ok(event),
            }
        }
    }

    impl From<Event1> for Events {
        fn from(event: Event1) -> Self {
            Events::Event1(event)
        }
    }

    #[tokio::test]
    async fn test_event_id() {
        let event_bus = EventBus::<Events>::new(1);

        let event_tx = event_bus.get_sender::<Event1>();

        let mut event_rx = event_bus.get_receiver::<Event1>();

        tokio::spawn(async move {
            event_tx.send(Event1 { id: 1 }).expect("send event");
        });

        let event = event_rx.recv().await.expect("receive event");

        assert_eq!(event, Event1 { id: 1 });
    }
}
