pub mod customers;
pub mod packages;

#[derive(Debug, Clone)]
pub struct Notifier(std::sync::mpsc::SyncSender<()>);
pub struct NotificationReceiver(std::sync::mpsc::Receiver<()>);

pub fn notifier() -> (Notifier, NotificationReceiver) {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    (Notifier(tx), NotificationReceiver(rx))
}

impl Notifier {
    /// Will attempt to notify the corresponding [`NotificationReceiver`], if there is already a
    /// pending notification, this will do nothing
    pub fn notify(&self) -> Result<(), ()> {
        match self.0.try_send(()) {
            Ok(_) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => Err(()),
        }
    }
}

impl NotificationReceiver {
    pub fn listen(&mut self) -> Result<(), ()> {
        match self.0.recv() {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}
