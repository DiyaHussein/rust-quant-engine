use crate::error::{QuantError, QuantResult};
use crate::types::Bar;
use chrono::NaiveDateTime;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::path::Path;

/// Raw CSV bar row
#[derive(Debug, Deserialize)]
struct CsvBar {
    timestamp: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// Load OHLCV bar data from a CSV file.
///
/// Expected CSV format: `timestamp,open,high,low,close,volume`
/// Timestamp format: `YYYY-MM-DD HH:MM:SS`
pub fn load_csv(path: impl AsRef<Path>) -> QuantResult<Vec<Bar>> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(path)?;

    let mut bars = Vec::new();

    for result in reader.deserialize::<CsvBar>() {
        let record = result?;
        let timestamp = NaiveDateTime::parse_from_str(&record.timestamp, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| {
                QuantError::Parse(format!("Invalid timestamp '{}': {}", record.timestamp, e))
            })?;

        bars.push(Bar {
            timestamp,
            open: record.open,
            high: record.high,
            low: record.low,
            close: record.close,
            volume: record.volume,
        });
    }

    Ok(bars)
}

/// Generate synthetic price data for testing.
///
/// Creates bars using a geometric Brownian motion (GBM) model.
pub fn generate_synthetic_bars(n: usize, start_price: f64, volatility: f64, seed: u64) -> Vec<Bar> {
    use rand::prelude::*;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(seed);
    let mut bars = Vec::with_capacity(n);
    let mut price = start_price;
    let base_time =
        NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for i in 0..n {
        let drift = 0.0;
        let shock: f64 = rng.sample(rand_distr::StandardNormal);
        let ret = drift + volatility * shock;
        price *= (1.0 + ret).max(0.001);

        let noise = volatility * price * 0.1;
        let high = price + noise.abs();
        let low = price - noise.abs();
        let volume = (rng.gen::<f64>().abs() * 10000.0) + 1000.0;

        bars.push(Bar {
            timestamp: base_time + chrono::Duration::minutes(i as i64),
            open: price,
            high,
            low,
            close: price,
            volume,
        });
    }

    bars
}
