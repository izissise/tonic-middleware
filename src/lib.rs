//! Export tonic middleware traits and macros

#![warn(missing_docs, unreachable_pub)]

mod middleware_trait;

pub use middleware_trait::TonicMiddleware;
pub use tonic_middleware_macros::tonic_middleware;


pub mod prelude {
    //! Re-exports important traits and types. Meant to be glob imported
    pub use crate::TonicMiddleware;
    pub use crate::tonic_middleware;
}
