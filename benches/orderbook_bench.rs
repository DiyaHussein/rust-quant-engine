use criterion::{criterion_group, criterion_main, Criterion};
use quant_engine::orderbook::OrderBook;
use quant_engine::types::Side;

fn bench_orderbook_submit_limit(c: &mut Criterion) {
    c.bench_function("orderbook_limit_order", |b| {
        let mut ob = OrderBook::new("BENCH");
        b.iter(|| {
            ob.submit_limit(Side::Buy, 100.0, 1.0).unwrap();
        });
    });
}

fn bench_orderbook_slippage_estimate(c: &mut Criterion) {
    c.bench_function("orderbook_slippage_estimate", |b| {
        let mut ob = OrderBook::new("BENCH");
        for i in 0..200 {
            ob.submit_limit(Side::Sell, 100.0 + i as f64 * 0.01, 10.0)
                .unwrap();
            ob.submit_limit(Side::Buy, 99.0 - i as f64 * 0.01, 10.0)
                .unwrap();
        }
        b.iter(|| {
            ob.estimate_slippage_bps(Side::Buy, 100.0);
        });
    });
}

criterion_group!(
    benches,
    bench_orderbook_submit_limit,
    bench_orderbook_slippage_estimate
);
criterion_main!(benches);
