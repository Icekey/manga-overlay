use ::serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct ChannelValue<T: Default> {
    #[serde(skip)]
    tx: Sender<T>,
    #[serde(skip)]
    rx: Receiver<T>,

    pub value: T,
}

impl<T: Default> Default for ChannelValue<T> {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel::<T>(32);
        Self {
            tx,
            rx,
            value: Default::default(),
        }
    }
}

impl<T: Default> ChannelValue<T> {
    pub fn tx(&self) -> Sender<T> {
        self.tx.clone()
    }

    pub fn update(&mut self) -> bool {
        if let Ok(value) = self.rx.try_recv() {
            self.value = value;
            return true;
        }
        return false;
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
    }

    pub fn reset(&mut self) {
        self.value = Default::default();
    }
}
