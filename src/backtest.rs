//! Event-driven backtesting engine.
//!
//! Simulates strategy execution against historical data with realistic
//! spread, slippage, and commission modeling.

#[cfg(test)]
use crate::data::generate_synthetic_bars;
use crate::metrics::Metrics;
use crate::orderbook::OrderBook;
use crate::types::*;
use std::collections::VecDeque;

/// Commission model
#[derive(Debug, Clone)]
pub enum CommissionModel {
    /// Fixed commission per trade
    FixedPerTrade(f64),
    /// Basis points per trade value
    BpsPerTrade(f64),
    /// No commission
    None,
}

impl CommissionModel {
    pub fn apply(&self, trade_value: f64) -> f64 {
        match self {
            CommissionModel::FixedPerTrade(fee) => *fee,
            CommissionModel::BpsPerTrade(bps) => trade_value * bps / 10000.0,
            CommissionModel::None => 0.0,
        }
    }
}

/// Slippage model
#[derive(Debug, Clone)]
pub enum SlippageModel {
    /// Fixed slippage in basis points
    FixedBps(f64),
    /// Dynamic: estimate from order book depth
    OrderBookDepth,
    /// No slippage
    None,
}

impl SlippageModel {
    pub fn apply(&self, _side: Side, _quantity: Quantity, ob: &OrderBook) -> f64 {
        match self {
            SlippageModel::FixedBps(bps) => *bps,
            SlippageModel::OrderBookDepth => ob.estimate_slippage_bps(_side, _quantity),
            SlippageModel::None => 0.0,
        }
    }
}

/// Configuration for a backtest run
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    /// Initial capital
    pub initial_capital: f64,
    /// Commission model
    pub commission: CommissionModel,
    /// Slippage model
    pub slippage: SlippageModel,
    /// Position size as fraction of capital (0.0-1.0)
    pub position_size_pct: f64,
    /// Whether to allow short selling
    pub allow_short: bool,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: 10_000.0,
            commission: CommissionModel::BpsPerTrade(1.0), // 1 bps
            slippage: SlippageModel::FixedBps(1.0),        // 1 bps
            position_size_pct: 1.0,
            allow_short: false,
        }
    }
}

/// Trade record from a backtest
#[derive(Debug, Clone)]
pub struct Trade {
    pub entry_time: chrono::NaiveDateTime,
    pub exit_time: chrono::NaiveDateTime,
    pub side: Side,
    pub entry_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub commission_paid: f64,
    pub slippage_paid: f64,
}

/// The backtest engine
pub struct Backtest {
    pub config: BacktestConfig,
    pub order_book: OrderBook,
    capital: f64,
    equity_curve: Vec<(chrono::NaiveDateTime, f64)>,
    trades: Vec<Trade>,
    position: f64,
    #[allow(dead_code)]
    pending_orders: VecDeque<Order>,
    trade_returns: Vec<f64>,
}

impl Backtest {
    /// Create a new backtest engine
    pub fn new(config: BacktestConfig, symbol: &str) -> Self {
        Self {
            capital: config.initial_capital,
            config: config.clone(),
            order_book: OrderBook::new(symbol),
            equity_curve: Vec::new(),
            trades: Vec::new(),
            position: 0.0,
            pending_orders: VecDeque::new(),
            trade_returns: Vec::new(),
        }
    }

    /// Run a backtest on bar data using a simple moving average crossover strategy.
    ///
    /// Returns the final Metrics.
    pub fn run_sma_crossover(
        &mut self,
        bars: &[Bar],
        fast_period: usize,
        slow_period: usize,
    ) -> Metrics {
        if bars.len() < slow_period {
            return Metrics::compute(&[], &[], 0.0);
        }

        let mut position_open = false;
        let mut entry_price: f64 = 0.0;
        let mut entry_bar: Option<&Bar> = None;

        for i in slow_period..bars.len() {
            let bar = &bars[i];

            // Compute SMAs
            let fast_sma = bars[i - fast_period..=i]
                .iter()
                .map(|b| b.close)
                .sum::<f64>()
                / (fast_period + 1) as f64;

            let slow_sma = bars[i - slow_period..=i]
                .iter()
                .map(|b| b.close)
                .sum::<f64>()
                / (slow_period + 1) as f64;

            let signal = fast_sma > slow_sma;

            if signal && !position_open {
                // BUY
                let price = bar.close;
                let slippage_bps = self.config.slippage.apply(Side::Buy, 1.0, &self.order_book);
                let exec_price = if matches!(self.config.slippage, SlippageModel::None) {
                    price
                } else {
                    price * (1.0 + slippage_bps / 10000.0)
                };

                let trade_value = self.capital * self.config.position_size_pct;
                let commission = self.config.commission.apply(trade_value);
                let quantity = (trade_value - commission) / exec_price;

                self.capital -= trade_value;
                self.position = quantity;
                position_open = true;
                entry_price = exec_price;
                entry_bar = Some(bar);
            } else if !signal && position_open {
                // SELL (close position)
                let price = bar.close;
                let slippage_bps = self
                    .config
                    .slippage
                    .apply(Side::Sell, 1.0, &self.order_book);
                let exec_price = if matches!(self.config.slippage, SlippageModel::None) {
                    price
                } else {
                    price * (1.0 - slippage_bps / 10000.0)
                };

                let trade_value = self.position * exec_price;
                let commission = self.config.commission.apply(trade_value);

                self.capital += trade_value - commission;
                let pnl = (exec_price - entry_price) * self.position - commission * 2.0;
                let pnl_pct = if entry_price > 0.0 {
                    ((exec_price / entry_price) - 1.0) * 100.0
                } else {
                    0.0
                };

                self.trades.push(Trade {
                    entry_time: entry_bar.map(|b| b.timestamp).unwrap_or(bar.timestamp),
                    exit_time: bar.timestamp,
                    side: Side::Buy,
                    entry_price,
                    exit_price: exec_price,
                    quantity: self.position,
                    pnl,
                    pnl_pct,
                    commission_paid: commission * 2.0,
                    slippage_paid: slippage_bps / 10000.0 * trade_value,
                });

                self.trade_returns.push(pnl_pct / 100.0);

                self.position = 0.0;
                position_open = false;
            }

            // Record equity
            let equity = self.capital + (self.position * bars[i].close);
            self.equity_curve.push((bar.timestamp, equity));
        }

        // Close any open position at last bar
        if position_open {
            let last = &bars[bars.len() - 1];
            let price = last.close;
            let trade_value = self.position * price;
            let commission = self.config.commission.apply(trade_value);
            self.capital += trade_value - commission;

            let _pnl = (price - entry_price) * self.position - commission * 2.0;
            let pnl_pct = if entry_price > 0.0 {
                ((price / entry_price) - 1.0) * 100.0
            } else {
                0.0
            };

            self.trade_returns.push(pnl_pct / 100.0);
            self.position = 0.0;
        }

        let equity_values: Vec<f64> = self.equity_curve.iter().map(|(_, v)| *v).collect();

        Metrics::compute(
            &self.trade_returns,
            &equity_values,
            252.0, // daily bars = 252 periods/year
        )
    }

    /// Get the equity curve as (timestamp, equity) pairs
    pub fn equity_curve(&self) -> &[(chrono::NaiveDateTime, f64)] {
        &self.equity_curve
    }

    /// Get all trades
    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    /// Get current capital
    pub fn capital(&self) -> f64 {
        self.capital
    }

    /// Get total number of trades
    pub fn total_trades(&self) -> usize {
        self.trades.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtest_no_commission() {
        let config = BacktestConfig {
            commission: CommissionModel::None,
            slippage: SlippageModel::None,
            initial_capital: 10_000.0,
            position_size_pct: 1.0,
            allow_short: false,
        };

        let mut bt = Backtest::new(config, "TEST");
        let bars = generate_synthetic_bars(500, 100.0, 0.01, 42);

        let metrics = bt.run_sma_crossover(&bars, 10, 50);
        println!(
            "Trades: {}, Sharpe: {:.2}, Sortino: {:.2}, Calmar: {:.2}, MaxDD: {:.1}%, WinRate: {:.0}%",
            bt.total_trades(),
            metrics.sharpe_ratio,
            metrics.sortino_ratio,
            metrics.calmar_ratio,
            metrics.max_drawdown_pct,
            metrics.win_rate_pct
        );

        // With random data, we should at least get some trades
        assert!(bt.total_trades() > 0);
    }

    #[test]
    fn test_backtest_with_commission() {
        let config = BacktestConfig {
            commission: CommissionModel::BpsPerTrade(2.0), // 2 bps per trade
            slippage: SlippageModel::FixedBps(1.0),
            initial_capital: 10_000.0,
            position_size_pct: 1.0,
            allow_short: false,
        };

        let mut bt = Backtest::new(config, "TEST");
        let bars = generate_synthetic_bars(200, 100.0, 0.02, 123);

        let metrics = bt.run_sma_crossover(&bars, 5, 20);
        assert!(bt.total_trades() > 0);
        assert!(metrics.profit_factor.is_finite());
    }
}
