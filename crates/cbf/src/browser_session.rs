use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{
    delegate::BackendDelegate,
    browser::{Backend, BrowserHandle, CommandSender, EventStream},
    error::Error,
};

/// A session that owns the initial command handle.
#[derive(Debug)]
pub struct BrowserSession<B: Backend> {
    handle: BrowserHandle<B>,
    closed: AtomicBool,
    next_shutdown_request_id: AtomicU64,
}

impl<B: Backend> BrowserSession<B> {
    pub(crate) fn new(command_tx: CommandSender<B>) -> Self {
        Self {
            handle: BrowserHandle::new(command_tx),
            closed: AtomicBool::new(false),
            next_shutdown_request_id: AtomicU64::new(1),
        }
    }

    /// Connect to a backend and obtain a command session and an event stream.
    ///
    /// This split form is the minimum core API: most applications want to drive
    /// the backend from one place, while consuming events elsewhere.
    pub fn connect<D: BackendDelegate>(
        backend: B,
        delegate: D,
        raw_delegate: Option<B::RawDelegate>,
    ) -> Result<(Self, EventStream<B>), Error> {
        let (command_tx, events) = backend.connect(delegate, raw_delegate)?;
        Ok((Self::new(command_tx), events))
    }

    /// Get a cloneable handle for issuing browser commands.
    pub fn handle(&self) -> BrowserHandle<B> {
        self.handle.clone()
    }

    /// Request a graceful shutdown flow once.
    ///
    /// This method is idempotent. If the backend is already disconnected,
    /// it is treated as already closed.
    pub fn close(&self) -> Result<(), Error> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let request_id = self
            .next_shutdown_request_id
            .fetch_add(1, Ordering::Relaxed);

        match self.handle.request_shutdown(request_id) {
            Ok(()) | Err(Error::Disconnected) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl<B: Backend> Drop for BrowserSession<B> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
