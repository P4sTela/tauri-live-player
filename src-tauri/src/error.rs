use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("GStreamer error: {0}")]
    GStreamer(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("Output error: {0}")]
    Output(String),

    #[error("Project error: {0}")]
    Project(String),

    #[error("File error: {0}")]
    File(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type AppResult<T> = Result<T, AppError>;
