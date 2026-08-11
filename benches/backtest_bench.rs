use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quant_engine::backtest::{Backtest, BacktestConfig, CommissionModel, SlippageModel};
use quant_engine::data::generate_synthetic_bars;
use quant_engine::orderbook::OrderBook;
use quant_engine::types::Side;

fn bench_orderbook_market_order(c: &mut Criterion) {
    c.bench_function("orderbook_market_order", |b| {
        let mut ob = OrderBook::new("BENCH");
        // Pre-seed the book with liquidity
        for i in 0..100 {
            ob.submit_limit(Side::Sell, 100.0 + i as f64 * 0.01, 10.0)
                .unwrap();
            ob.submit_limit(Side::Buy, 99.0 - i as f64 * 0.01, 10.0)
                .unwrap();
        }
        b.iter(|| {
            ob.submit_market(Side::Buy, 1.0).unwrap();
        });
    });
}

fn bench_backtest_sma(c: &mut Criterion) {
    c.bench_function("backtest_sma_1000_bars", |b| {
        let bars = generate_synthetic_bars(1000, 100.0, 0.02, 42);
        let config = BacktestConfig {
            commission: CommissionModel::BpsPerTrade(1.0),
            slippage: SlippageModel::FixedBps(1.0),
            ..Default::default()
        };
        b.iter(|| {
            let mut bt = Backtest::new(config.clone(), "BENCH");
            black_box(bt.run_sma_crossover(&bars, 10, 50));
        });
    });
}

criterion_group!(benches, bench_orderbook_market_order, bench_backtest_sma);
criterion_main!(benches);
