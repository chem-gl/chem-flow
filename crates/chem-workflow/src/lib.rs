//! chem-workflow: motores de flujo quimicos
//!
//! Crate inicial que define los traits y estructuras principales para
//! implementar motores de flujo quimicos (ChemicalFlowEngine) que usan
//! `flow::FlowRepository` y `chem_domain::DomainRepository`.

pub mod config;
pub mod engine;
pub mod errors;
pub mod factory;
pub mod flows;
pub mod persistence;
pub mod step;
pub mod workflow_type;

pub use config::WorkflowConfig;
pub use engine::ChemicalFlowEngine;
pub use errors::WorkflowError;
pub use step::WorkflowStep;
pub use workflow_type::WorkflowType;
