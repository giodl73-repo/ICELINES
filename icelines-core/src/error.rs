use thiserror::Error;

#[derive(Debug, Error)]
pub enum IcelinesError {
    #[error("unknown team abbreviation: {0}")]
    UnknownTeam(String),

    #[error("cannot parse position string: {0}")]
    InvalidPosition(String),

    #[error("GP is zero — cannot compute pace score")]
    ZeroGp,

    #[error("GP {gp} is below minimum threshold {min}")]
    BelowMinGp { gp: u32, min: u32 },
}
