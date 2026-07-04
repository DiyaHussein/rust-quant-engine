/// Financial performance metrics.
///
/// All metrics are computed from a series of equity values or trade returns.
use ndarray::Array1;
use statrs::statistics::Statistics;

/// Comprehensive performance metrics for a backtest run.
#[derive(Debug, Clone)]
pub struct Metrics {
    /// Total return (%)
    pub total_return_pct: f64,
    /// Annualized return (%)
    pub annualized_return_pct: f64,
    /// Annualized volatility (%)
    pub annualized_volatility_pct: f64,
    /// Sharpe ratio (risk-free rate = 0 by default)
    pub sharpe_ratio: f64,
    /// Sortino ratio (downside deviation only)
    pub sortino_ratio: f64,
    /// Calmar ratio (annualized return / max drawdown)
    pub calmar_ratio: f64,
    /// Maximum drawdown (%)
    pub max_drawdown_pct: f64,
    /// Total number of trades
    pub total_trades: usize,
    /// Number of winning trades
    pub winning_trades: usize,
    /// Number of losing trades
    pub losing_trades: usize,
    /// Win rate (%)
    pub win_rate_pct: f64,
    /// Average win (%)
    pub avg_win_pct: f64,
    /// Average loss (%)
    pub avg_loss_pct: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
}

/// Summary of core metrics (compact, for display)
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub max_dd_pct: f64,
    pub win_rate_pct: f64,
    pub profit_factor: f64,
    pub total_return_pct: f64,
}

impl Metrics {
    /// Compute metrics from a series of trade returns (as decimals: 0.01 = 1%).
    ///
    /// `returns` should be the return of each trade relative to entry.
    /// `equity_curve` is the equity value at each bar.
    /// `periods_per_year` scales annualization (e.g., 252 for daily bars).
    pub fn compute(
        returns: &[f64],
        equity_curve: &[f64],
        periods_per_year: f64,
    ) -> Self {
        if returns.is_empty() || equity_curve.len() < 2 {
            return Self::empty();
        }

        let rets = Array1::from_vec(returns.to_vec());

        let winning: Vec<f64> = rets.iter().copied().filter(|&r| r > 0.0).collect();
        let losing: Vec<f64> = rets.iter().copied().filter(|&r| r < 0.0).collect();

        let total_return_pct = compute_total_return(equity_curve);
        let annualized_return_pct = compute_annualized_return(equity_curve, periods_per_year);
        let annualized_vol_pct = compute_annualized_volatility(equity_curve, periods_per_year);
        let max_dd_pct = compute_max_drawdown(equity_curve);

        let sharpe = if annualized_vol_pct > 0.0 {
            annualized_return_pct / annualized_vol_pct
        } else {
            0.0
        };

        let sortino = compute_sortino(equity_curve, periods_per_year);
        let calmar = if max_dd_pct > 0.0 {
            annualized_return_pct / max_dd_pct
        } else {
            0.0
        };

        let gross_profit: f64 = winning.iter().sum::<f64>().abs();
        let gross_loss: f64 = losing.iter().sum::<f64>().abs();

        Metrics {
            total_return_pct,
            annualized_return_pct,
            annualized_volatility_pct: annualized_vol_pct,
            sharpe_ratio: sharpe,
            sortino_ratio: sortino,
            calmar_ratio: calmar,
            max_drawdown_pct: max_dd_pct,
            total_trades: returns.len(),
            winning_trades: winning.len(),
            losing_trades: losing.len(),
            win_rate_pct: if returns.is_empty() { 0.0 } else {
                (winning.len() as f64 / returns.len() as f64) * 100.0
            },
            avg_win_pct: if !winning.is_empty() {
                winning.mean() * 100.0
            } else {
                0.0
            },
            avg_loss_pct: if !losing.is_empty() {
                losing.mean().abs() * 100.0
            } else {
                0.0
            },
            profit_factor: if gross_loss > 0.0 {
                gross_profit / gross_loss
            } else if gross_profit > 0.0 {
                f64::INFINITY
            } else {
                0.0
            },
        }
    }

    fn empty() -> Self {
        Metrics {
            total_return_pct: 0.0,
            annualized_return_pct: 0.0,
            annualized_volatility_pct: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            calmar_ratio: 0.0,
            max_drawdown_pct: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate_pct: 0.0,
            avg_win_pct: 0.0,
            avg_loss_pct: 0.0,
            profit_factor: 0.0,
        }
    }

    /// Compact summary for display
    pub fn summary(&self) -> MetricsSummary {
        MetricsSummary {
            sharpe: self.sharpe_ratio,
            sortino: self.sortino_ratio,
            calmar: self.calmar_ratio,
            max_dd_pct: self.max_drawdown_pct,
            win_rate_pct: self.win_rate_pct,
            profit_factor: self.profit_factor,
            total_return_pct: self.total_return_pct,
        }
    }
}

/// Total return from an equity curve (%)
pub fn compute_total_return(equity: &[f64]) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let start = equity[0];
    let end = equity[equity.len() - 1];
    if start <= 0.0 {
        return 0.0;
    }
    ((end / start) - 1.0) * 100.0
}

/// Annualized return from daily returns (%)
pub fn compute_annualized_return(equity: &[f64], periods_per_year: f64) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }
    let total_return = compute_total_return(equity) / 100.0 + 1.0;
    let years = (equity.len() - 1) as f64 / periods_per_year;
    if years <= 0.0 {
        return 0.0;
    }
    (total_return.powf(1.0 / years) - 1.0) * 100.0
}

/// Annualized volatility from log returns (%)
pub fn compute_annualized_volatility(equity: &[f64], periods_per_year: f64) -> f64 {
    if equity.len() < 3 {
        return 0.0;
    }
    let log_rets: Vec<f64> = equity
        .windows(2)
        .filter(|w| w[0] > 0.0 && w[1] > 0.0)
        .map(|w| (w[1] / w[0]).ln())
        .collect();

    if log_rets.is_empty() {
        return 0.0;
    }

    let arr = Array1::from_vec(log_rets);
    arr.std_dev() * periods_per_year.sqrt() * 100.0
}

/// Maximum drawdown (%)
pub fn compute_max_drawdown(equity: &[f64]) -> f64 {
    if equity.len() < 2 {
        return 0.0;
    }

    let mut peak = equity[0];
    let mut max_dd = 0.0_f64;

    for &value in &equity[1..] {
        if value > peak {
            peak = value;
        }
        let dd = (peak - value) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    max_dd * 100.0
}

/// Sortino ratio: annualized return / downside deviation
pub fn compute_sortino(equity: &[f64], periods_per_year: f64) -> f64 {
    if equity.len() < 3 {
        return 0.0;
    }

    let ann_return = compute_annualized_return(equity, periods_per_year);

    let log_rets: Vec<f64> = equity
        .windows(2)
        .filter(|w| w[0] > 0.0 && w[1] > 0.0)
        .map(|w| (w[1] / w[0]).ln())
        .filter(|&r| r < 0.0) // downside only
        .collect();

    if log_rets.is_empty() {
        return if ann_return > 0.0 { f64::INFINITY } else { 0.0 };
    }

    let arr = Array1::from_vec(log_rets);
    let downside_dev = arr.std_dev() * periods_per_year.sqrt() * 100.0;

    if downside_dev > 0.0 {
        ann_return / downside_dev
    } else if ann_return > 0.0 {
        f64::INFINITY
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_drawdown() {
        let equity = vec![100.0, 90.0, 95.0, 85.0, 100.0];
        let dd = compute_max_drawdown(&equity);
        // Peak: 100, trough: 85 => DD = 15%
        assert!((dd - 15.0).abs() < 0.01, "Got dd={}", dd);
    }

    #[test]
    fn test_total_return() {
        let equity = vec![100.0, 110.0, 105.0, 120.0];
        let ret = compute_total_return(&equity);
        assert!((ret - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_win_rate() {
        let returns = vec![0.01, -0.005, 0.02, -0.01, 0.015];
        let equity = vec![100.0, 101.0, 100.5, 102.5, 101.5, 103.0];
        let m = Metrics::compute(&returns, &equity, 252.0);
        assert_eq!(m.winning_trades, 3);
        assert_eq!(m.losing_trades, 2);
    }
}
