# rust-quant-engine

[![CI](https://github.com/DiyaHussein/rust-quant-engine/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/DiyaHussein/rust-quant-engine/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rust-quant-engine)](https://crates.io/crates/rust-quant-engine)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**High-performance event-driven backtesting engine in Rust with Python bindings.**

Built for quantitative researchers and algorithmic traders who need speed. Processes tick-level market data with realistic order book simulation, slippage modeling, and professional-grade performance metrics.

## Features

- **Limit Order Book** — Full LOB with bid/ask spread, depth tracking, and market impact estimation
- **Event-Driven Backtester** — Tick-by-tick simulation with configurable commission and slippage models
- **Performance Metrics** — Sharpe, Sortino, Calmar ratios, max drawdown, win rate, profit factor
- **Python Bindings** — PyO3-powered bindings for use from Python trading scripts (in progress)
- **Synthetic Data Generator** — Geometric Brownian motion for quick strategy prototyping

## Quick Start

```rust
use quant_engine::backtest::{Backtest, BacktestConfig};
use quant_engine::data::generate_synthetic_bars;

fn main() {
    // Generate 1,000 bars of synthetic price data
    let bars = generate_synthetic_bars(1000, 100.0, 0.02, 42);

    // Configure backtest with realistic costs
    let config = BacktestConfig {
        initial_capital: 10_000.0,
        commission: CommissionModel::BpsPerTrade(1.0), // 1 bps
        slippage: SlippageModel::FixedBps(1.0),        // 1 bps
        ..Default::default()
    };

    // Run SMA crossover strategy
    let mut bt = Backtest::new(config, "XAUUSD");
    let metrics = bt.run_sma_crossover(&bars, 10, 50);

    println!("Sharpe: {:.2}", metrics.sharpe_ratio);
    println!("Sortino: {:.2}", metrics.sortino_ratio);
    println!("Calmar: {:.2}", metrics.calmar_ratio);
    println!("Max Drawdown: {:.1}%", metrics.max_drawdown_pct);
    println!("Win Rate: {:.0}%", metrics.win_rate_pct);
    println!("Profit Factor: {:.2}", metrics.profit_factor);
    println!("Total Trades: {}", metrics.total_trades);
}
```

## Installation

### Rust

```toml
[dependencies]
rust-quant-engine = "0.1"
```

### Python (coming soon)

```bash
pip install rust-quant-engine
```

## Benchmarks

Target: **50x faster** than Python-native backtesters (backtrader, vectorbt) on tick-level data.

| Benchmark | rust-quant-engine | backtrader | vectorbt |
|-----------|-------------------|------------|----------|
| OrderBook market order (1M ops) | _pending_ | - | - |
| SMA crossover (10K bars) | _pending_ | - | - |
| Slippage estimation (100K) | _pending_ | - | - |

_Run benchmarks:_ `cargo bench`

## Architecture

```
src/
├── lib.rs          # Crate root, re-exports
├── orderbook.rs    # Limit order book with LOB simulation
├── backtest.rs     # Event-driven backtesting engine
├── metrics.rs      # Sharpe, Sortino, Calmar, max drawdown
├── types.rs        # Core types (Order, Fill, Bar, Side)
├── data.rs         # CSV loader + synthetic data generator
└── error.rs        # Error types
```

## Roadmap

- [x] Core order book matching engine
- [x] SMA crossover strategy backtest
- [x] Performance metrics suite
- [x] Synthetic data generator
- [ ] Python bindings via PyO3
- [ ] Multi-asset portfolio backtesting
- [ ] Custom strategy API (trait-based)
- [ ] Real-time market data feed integration
- [ ] WebAssembly target for browser-based visualization

## License

MIT — see [LICENSE](LICENSE) for details.
