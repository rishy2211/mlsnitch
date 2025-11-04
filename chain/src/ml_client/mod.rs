//! Clients for the external ML verification service.
//!
//! This module provides concrete implementations of the generic
//! [`crate::validation::MlVerifier`] trait. These clients are responsible
//! for talking to the Python + PyTorch watermarking service over HTTP/gRPC
//! and translating responses into [`crate::validation::MlVerdict`] values.

pub mod http;

pub use http::HttpMlVerifier;
