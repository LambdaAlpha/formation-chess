//! Core rules engine for Formation Chess: board, pieces, abilities,
//! formations, game flow, and the Chinese text notation protocol. See
//! docs/rules.md (repo root) for the rules and docs/notation.md for
//! the protocol.

/// Package version of the rules engine.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod notation;

pub mod game;

pub mod action;

pub mod board;

pub mod piece;

pub mod formation;

pub mod ability;

pub(crate) mod chinese_num;
