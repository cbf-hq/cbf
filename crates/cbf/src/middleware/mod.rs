//! Middleware composition utilities for backend delegates.
//!
//! In normal operation, include [`lifecycle::LifecycleLayer`] in every middleware stack.
//! The lifecycle layer resolves stale `beforeunload` dialog requests and prevents
//! leaked dialog state when pages close or the backend tears down.
//!
//! Recommended layers for production:
//! - [`lifecycle::LifecycleLayer`] (required by default)
//! - [`auto_dialog_responder::AutoDialogResponderLayer`] (recommended safeguard)
//! - [`logging::LoggingLayer`] (recommended observability)
//!
//! If you intentionally run without lifecycle cleanup, call
//! [`MiddlewareBuilder::allow_unsafe_no_lifecycle`] explicitly.

pub mod auto_dialog_responder;
pub mod error_guard;
pub mod lifecycle;
pub mod logging;

use crate::{
    backend_delegate::{
        BackendDelegate, CommandDecision, DelegateContext, EventDecision, NoopDelegate,
    },
    error::{Error, InvalidConfiguration},
    event::BackendStopReason,
};

pub trait DelegateLayer: Send + 'static {
    /// Wraps `inner` and returns the composed delegate.
    fn wrap(self: Box<Self>, inner: Box<dyn BackendDelegate>) -> Box<dyn BackendDelegate>;

    /// Marks whether this layer provides lifecycle safety behavior.
    fn is_lifecycle(&self) -> bool {
        false
    }
}

/// Builder for composing middleware layers into a single delegate.
pub struct MiddlewareBuilder {
    layers: Vec<Box<dyn DelegateLayer>>,
    allow_unsafe_no_lifecycle: bool,
}

impl MiddlewareBuilder {
    /// Creates an empty middleware builder.
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            allow_unsafe_no_lifecycle: false,
        }
    }

    /// Appends a layer to the stack.
    ///
    /// Layers are wrapped in insertion order.
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: DelegateLayer,
    {
        self.layers.push(Box::new(layer));
        self
    }

    /// Allows building a stack without [`lifecycle::LifecycleLayer`].
    ///
    /// This is unsafe by policy and should only be enabled intentionally.
    pub fn allow_unsafe_no_lifecycle(mut self, allow: bool) -> Self {
        self.allow_unsafe_no_lifecycle = allow;
        self
    }

    /// Builds the composed middleware delegate.
    ///
    /// Returns [`Error::InvalidConfiguration`] when lifecycle protection is missing
    /// and unsafe mode is not explicitly enabled.
    pub fn build(self) -> Result<MiddlewareDelegate, Error> {
        // Ensure that a LifecycleLayer is present unless the user has explicitly allowed it to be missing.
        if !self.allow_unsafe_no_lifecycle && !self.layers.iter().any(|layer| layer.is_lifecycle())
        {
            return Err(Error::InvalidConfiguration(
                InvalidConfiguration::MissingLifecycleLayer,
            ));
        }

        let mut delegate: Box<dyn BackendDelegate> = Box::new(NoopDelegate);
        for layer in self.layers {
            delegate = layer.wrap(delegate);
        }

        Ok(MiddlewareDelegate { inner: delegate })
    }
}

impl Default for MiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Backend delegate produced by [`MiddlewareBuilder`].
pub struct MiddlewareDelegate {
    inner: Box<dyn BackendDelegate>,
}

impl BackendDelegate for MiddlewareDelegate {
    fn on_command(
        &mut self,
        ctx: &mut DelegateContext,
        command: &crate::command::BrowserCommand,
    ) -> CommandDecision {
        self.inner.on_command(ctx, command)
    }

    fn on_event(
        &mut self,
        ctx: &mut DelegateContext,
        event: &crate::event::BrowserEvent,
    ) -> EventDecision {
        self.inner.on_event(ctx, event)
    }

    fn on_idle(&mut self, ctx: &mut DelegateContext) {
        self.inner.on_idle(ctx)
    }

    fn on_teardown(&mut self, ctx: &mut DelegateContext, reason: BackendStopReason) {
        self.inner.on_teardown(ctx, reason)
    }
}
