use std::error::Error;
use std::fmt;

pub trait Embedder: Send + Sync {
    fn identity(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError>;
}

#[derive(Debug)]
pub enum EmbedError {
    Unavailable(Box<dyn Error + Send + Sync>),
    InvalidVector(&'static str),
}

impl EmbedError {
    pub fn unavailable<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self::Unavailable(Box::new(error))
    }
}

impl fmt::Display for EmbedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(error) => write!(f, "embedding unavailable: {error}"),
            Self::InvalidVector(message) => f.write_str(message),
        }
    }
}

impl Error for EmbedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Unavailable(error) => Some(error.as_ref()),
            Self::InvalidVector(_) => None,
        }
    }
}

pub fn validate_embedding(vector: &[f32]) -> Result<(), EmbedError> {
    if vector.is_empty() {
        return Err(EmbedError::InvalidVector(
            "embedding vectors must not be empty",
        ));
    }

    let mut norm = 0.0f32;

    for value in vector {
        if !value.is_finite() {
            return Err(EmbedError::InvalidVector(
                "embedding vectors must contain only finite values",
            ));
        }

        norm += value * value;
    }

    if norm == 0.0 {
        return Err(EmbedError::InvalidVector(
            "embedding vectors must have non-zero magnitude",
        ));
    }

    Ok(())
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32, EmbedError> {
    validate_embedding(left)?;
    validate_embedding(right)?;

    if left.len() != right.len() {
        return Err(EmbedError::InvalidVector(
            "embedding vectors must have the same dimension",
        ));
    }

    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }

    Ok(dot / (left_norm.sqrt() * right_norm.sqrt()))
}
