//! Bot module - Core bot functionality.

pub mod dispatcher;
mod runtime;

pub use dispatcher::build_dispatcher;
pub use runtime::run;
