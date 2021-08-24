use std::error::Error;
use std::fmt;
use lyon::lyon_tessellation::TessellationError;
use crate::engine::error::EngineError::TesselationError;

#[derive(Debug)]
pub enum EngineError {
    UnknownError,
    TesselationError(lyon::tessellation::TessellationError)
}

impl From<lyon::tessellation::TessellationError> for EngineError {
    fn from(te: TessellationError) -> Self {
        EngineError::TesselationError(te)
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::UnknownError => write!(f, "unknown error"),
            EngineError::TesselationError(te) => write!(f, "tesselation error: {:?}", te)
        }

    }
}

impl Error for EngineError
{
    // fn source(&self) -> Option<&(dyn Error + 'static)> {
    //     Some(&self.side)
    // }
}