//! # Observability
//! This crate provides helper functions to easily instrument server apps with
//! observability.
//!
//! ## Setup
//! Tracing, metrics and logs can be set up using [`setup::setup`]. This should
//! be the first call of any server binary.
//!
//! ## Http Trace Propagation
//! [`http_injector`] provides functions for injecting and extracting tracing
//! data into/from [`http::Request`]s.
//!
//! When using [`tower`] based http frameworks like [`axum`](https://docs.rs/axum/latest/axum), the middleware [`middleware::tower::TraceLayer`] can
//! be used to handle the extraction parts of http requests, correlating traces
//! across different services.
//!
//! Generally, the middleware should be used on every http route, this usually
//! means that it can be registered globally and in the last position, to be the
//! first to run.

pub mod http_injector;
pub mod middleware;
pub mod setup;
pub mod trace_id;
