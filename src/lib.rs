//! # rust-quant-engine
//!
//! High-performance event-driven backtesting engine in Rust.
//!
//! ## Features
//!
//! - **Limit Order Book**: Full LOB simulation with bid/ask spread, slippage, and market impact
//! - **Event-driven backtester**: Tick-by-tick simulation with realistic fill modeling
//! - **Performance metrics**: Sharpe ratio, Sortino ratio, max drawdown, Calmar ratio, win rate, profit factor
//! - **Python bindings**: PyO3-powered bindings for use from Python trading scripts
//!
//! ## Quick Start
//!
//! ```rust
//! use quant_engine::{Backtest, OrderBook, Metrics};
//! ```
//!
//! ## Benchmarks
//!
//! See `benchmarks/` for detailed performance comparisons against backtrader and vectorbt.
//! Target: 50x faster than Python-native backtesters on tick-level data.

pub mod orderbook;
pub mod backtest;
pub mod metrics;
pub mod types;
pub mod data;
pub mod error;

pub use orderbook::OrderBook;
pub use backtest::{Backtest, BacktestConfig};
pub use metrics::{Metrics, MetricsSummary};
pub use types::*;
pub use data::*;
pub use error::QuantError;
