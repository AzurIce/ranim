//! Shared lifetime guards for dynamically loaded code.

use std::{any::Any, sync::Arc};

/// Retained owners, released in reverse registration order.
///
/// A library can load another library whose guard's destructor belongs to the
/// first library. Releasing newer guards first keeps that destructor loaded.
/// Iteration and cloning preserve registration order so every lifetime carrier
/// retains the same dependency order.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct KeepAlive(Vec<Arc<dyn Any + Send + Sync>>);

impl KeepAlive {
    /// Retain an owner until this container is destroyed.
    pub fn push(&mut self, owner: Arc<dyn Any + Send + Sync>) {
        self.0.push(owner);
    }

    /// Visit owners in registration order.
    pub fn iter(&self) -> std::slice::Iter<'_, Arc<dyn Any + Send + Sync>> {
        self.0.iter()
    }
}

impl Drop for KeepAlive {
    fn drop(&mut self) {
        while let Some(owner) = self.0.pop() {
            drop(owner);
        }
    }
}
