use std::fmt::Display;

/// An agent analysis or turn-execution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// The agent could not produce an analysis.
    Decision(String),
    /// The agent returned candidates that violated the analysis contract.
    InvalidAnalysis(String),
    /// The framework was called for a game state in which no agent turn exists.
    GameState(String),
}

impl Display for AgentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decision(message) => write!(formatter, "agent analysis failed: {message}"),
            Self::InvalidAnalysis(message) => {
                write!(formatter, "agent returned an invalid analysis: {message}")
            },
            Self::GameState(message) => write!(formatter, "agent turn unavailable: {message}"),
        }
    }
}

impl std::error::Error for AgentError {}
