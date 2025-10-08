//! Módulo `engine` (compatibilidad): reexporta los tipos principales definidos
//! en `domain`.
//!
//! Este módulo evita duplicación de definiciones y sirve como fachada para
//! código heredado que importaba `flow::engine::*`.

pub use crate::domain::{FlowData, FlowMeta, PersistResult, SnapshotMeta, WorkItem};
