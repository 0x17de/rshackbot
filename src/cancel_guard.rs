pub struct CancelGuard {
    sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl CancelGuard {
    pub fn new(sender: tokio::sync::oneshot::Sender<()>) -> CancelGuard {
        CancelGuard{ sender: Some(sender) }
    }

    pub fn cancel(mut self) {
        if self.sender.is_some() {
            let _ = std::mem::replace(&mut self.sender, None).unwrap().send(());
        }
    }
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        if self.sender.is_some() {
            let _ = std::mem::replace(&mut self.sender, None).unwrap().send(());
        }
    }
}
