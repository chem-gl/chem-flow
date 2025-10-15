//! Módulo de handlers HTTP

pub mod cadma_handlers;
pub mod family_handlers;
pub mod molecule_handlers;
pub mod property_handlers;
pub mod team_handlers;
pub mod user_handlers;

pub use cadma_handlers::*;
#[allow(unused_imports)]
pub use molecule_handlers::*;
#[allow(unused_imports)]
pub use property_handlers::*;
#[allow(unused_imports)]
pub use team_handlers::*;
#[allow(unused_imports)]
pub use user_handlers::*;
