use std::sync::{Mutex, MutexGuard};

static I18N_RUNTIME_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn i18n_runtime_guard() -> MutexGuard<'static, ()> {
    I18N_RUNTIME_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
