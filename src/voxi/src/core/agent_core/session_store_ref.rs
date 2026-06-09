/// RAII guard providing access to the SessionStore while holding the lock.
pub struct SessionStoreRef<'a> {
    guard: std::sync::MutexGuard<'a, Option<SessionStore>>,
}

impl<'a> SessionStoreRef<'a> {
    pub fn store(&self) -> Result<&SessionStore, String> {
        self.guard.as_ref().ok_or_else(|| "SessionStore not initialized".to_string())
    }
}

