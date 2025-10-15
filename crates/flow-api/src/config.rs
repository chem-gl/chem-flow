//! Configuración de la aplicación desde variables de entorno

use once_cell::sync::Lazy;
use std::env;

/// Configuración global de la aplicación
#[derive(Debug, Clone)]
pub struct AppConfig {
  /// URL de conexión a la base de datos
  pub database_url: String,

  /// Puerto del servidor HTTP
  pub port: u16,

  /// Host del servidor
  pub host: String,

  /// Nivel de logging
  pub log_level: String,

  /// Indica si está en modo desarrollo
  pub is_dev: bool,

  /// Secreto para firmar los JWT
  pub jwt_secret: String,
}

impl AppConfig {
  /// Carga la configuración desde variables de entorno
  pub fn from_env() -> anyhow::Result<Self> {
    dotenvy::dotenv().ok(); // Cargar .env si existe

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
                                                 // Default a SQLite para desarrollo
                                                 "file::memory:?mode=memory&cache=shared".to_string()
                                               });

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse().unwrap_or(3000);

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info,flow_api=debug".to_string());

    let is_dev = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()).to_lowercase() == "development";

    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret-for-dev".to_string());

    Ok(Self { database_url, port, host, log_level, is_dev, jwt_secret })
  }

  /// Dirección completa del servidor
  pub fn server_address(&self) -> String {
    format!("{}:{}", self.host, self.port)
  }
}

/// Instancia global de configuración (lazy-loaded)
pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| AppConfig::from_env().expect("Error cargando configuración"));
