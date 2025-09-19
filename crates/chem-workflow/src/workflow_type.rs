use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Enum para identificar los tipos de workflow que soporta el crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowType {
    Cadma,
    Unknown,
}

impl fmt::Display for WorkflowType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            WorkflowType::Cadma => "cadma",
            WorkflowType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for WorkflowType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cadma" => Ok(WorkflowType::Cadma),
            _ => Ok(WorkflowType::Unknown),
        }
    }
}

impl Default for WorkflowType {
    fn default() -> Self {
        WorkflowType::Unknown
    }
}
