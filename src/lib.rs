//! Export tonic middleware traits and macros

#![warn(missing_docs, unreachable_pub)]

mod middleware_trait;
mod rpc_status_error;

pub use middleware_trait::TonicMiddleware;
pub use rpc_status_error::RPCStatusError;
pub use tonic_middleware_macros::tonic_middleware;

pub mod prelude {
    //! Re-exports important traits and types. Meant to be glob imported
    pub use crate::tonic_middleware;
    pub use crate::TonicMiddleware;
}
