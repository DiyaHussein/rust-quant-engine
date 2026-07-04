use thiserror::Error;

#[derive(Error, Debug)]
pub enum QuantError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Backtest error: {0}")]
    Backtest(String),

    #[error("Order rejected: {0}")]
    OrderRejected(String),
}

pub type QuantResult<T> = Result<T, QuantError>;
