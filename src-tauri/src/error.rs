use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
  #[error("io error: {0}")]
  Io(#[from] std::io::Error),
  #[error("json parse error: {0}")]
  Json(#[from] serde_json::Error),
  #[error("invalid state: {0}")]
  InvalidState(String),
}
