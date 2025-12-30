//! Bot module - Core bot functionality.

pub mod dispatcher;
mod runtime;
pub mod webhook;

pub use dispatcher::build_dispatcher;
pub use runtime::run;
