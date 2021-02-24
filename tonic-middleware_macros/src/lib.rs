//! Generates tonic services with middlewares wrapper

#![recursion_limit = "256"]
#![warn(missing_docs, unreachable_pub)]


use proc_macro::TokenStream;

mod tonic_middleware_macro;
use tonic_middleware_macro::tonic_middleware as tonic_middleware_impl;

use proc_macro_error::proc_macro_error;

/// Public usable macros
#[proc_macro_attribute]
#[proc_macro_error]
pub fn tonic_middleware(args: TokenStream, item: TokenStream) -> TokenStream {
    tonic_middleware_impl(args, item)
}
