#![forbid(unsafe_code)]

//! UI-independent functionality for preparing boot media.

pub mod macos;

#[cfg(test)]
mod write_prototype;
