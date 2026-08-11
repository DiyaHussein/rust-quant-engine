//! Limit Order Book with realistic market microstructure.
//!
//! Uses BTreeMap for sorted price levels (bids descending, asks ascending).
//! Includes spread modeling, slippage estimation, and market impact.

use crate::error::QuantResult;
use crate::types::*;
use std::collections::BTreeMap;

/// Price level in the order book
#[derive(Debug, Clone, Default)]
pub struct PriceLevel {
    /// Total volume at this price (sum of all orders)
    pub total_volume: Quantity,
    /// Individual orders at this level
    pub orders: Vec<Order>,
}

/// A limit order book supporting market, limit, and stop orders.
///
/// Bids are sorted descending (highest willingness to buy first).
/// Asks are sorted ascending (lowest willingness to sell first).
#[derive(Debug, Clone)]
pub struct OrderBook {
    /// Instrument name (e.g., "XAUUSD")
    pub symbol: String,
    /// Bid side: price -> level (sorted descending)
    bids: BTreeMap<OrderKey, PriceLevel>,
    /// Ask side: price -> level (sorted ascending)
    asks: BTreeMap<OrderKey, PriceLevel>,
    /// Current mid-price (last trade price)
    pub last_price: Price,
    /// All fills generated
    pub fills: Vec<Fill>,
    /// Order ID counter
    next_id: OrderId,
    /// Minimum tick size
    pub tick_size: Price,
    /// Estimated spread in price units
    pub estimated_spread: Price,
    /// Order book depth for slippage calculation
    pub depth_bps: f64,
}

/// Wrapper for reverse ordering (bids: high-to-low)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct OrderKey {
    price_int: i64,
}

impl OrderKey {
    fn new(price: Price, tick_size: Price) -> Self {
        Self {
            price_int: (price / tick_size).round() as i64,
        }
    }
}

impl OrderBook {
    /// Create a new order book for a given symbol.
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_price: 0.0,
            fills: Vec::new(),
            next_id: 1,
            tick_size: 0.01,
            estimated_spread: 0.0,
            depth_bps: 5.0, // 5 bps per level of depth
        }
    }

    /// Submit a limit order. Returns the order with its assigned ID.
    pub fn submit_limit(
        &mut self,
        side: Side,
        price: Price,
        quantity: Quantity,
    ) -> QuantResult<Order> {
        let id = self.next_id;
        self.next_id += 1;
        let order = Order {
            id,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            stop_price: None,
            quantity,
            filled: 0.0,
            status: OrderStatus::Pending,
            timestamp: chrono::Utc::now().naive_utc(),
        };
        self.add_to_book(&order);
        Ok(order)
    }

    /// Submit a market order. Returns fills generated.
    pub fn submit_market(&mut self, side: Side, quantity: Quantity) -> QuantResult<Vec<Fill>> {
        let id = self.next_id;
        self.next_id += 1;
        let order = Order {
            id,
            side,
            order_type: OrderType::Market,
            price: None,
            stop_price: None,
            quantity,
            filled: 0.0,
            status: OrderStatus::Pending,
            timestamp: chrono::Utc::now().naive_utc(),
        };
        self.match_order(order)
    }

    /// Cancel an active order by ID
    pub fn cancel_order(&mut self, order_id: OrderId) -> QuantResult<()> {
        // Check bids
        for level in self.bids.values_mut() {
            if let Some(pos) = level.orders.iter().position(|o| o.id == order_id) {
                level.orders[pos].status = OrderStatus::Cancelled;
                level.total_volume -= level.orders[pos].remaining();
                level.orders.remove(pos);
                // Clean up empty levels
                return Ok(());
            }
        }
        // Check asks
        for level in self.asks.values_mut() {
            if let Some(pos) = level.orders.iter().position(|o| o.id == order_id) {
                level.orders[pos].status = OrderStatus::Cancelled;
                level.total_volume -= level.orders[pos].remaining();
                level.orders.remove(pos);
                return Ok(());
            }
        }
        Ok(())
    }

    /// Get the best bid price
    pub fn best_bid(&self) -> Option<Price> {
        // Bids are stored with OrderKey; highest original price = first
        // Since OrderKey is ascending, we want the last (max) element
        self.bids
            .keys()
            .last()
            .map(|k| k.price_int as f64 * self.tick_size)
    }

    /// Get the best ask price
    pub fn best_ask(&self) -> Option<Price> {
        self.asks
            .keys()
            .next()
            .map(|k| k.price_int as f64 * self.tick_size)
    }

    /// Get the mid price
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    /// Estimate slippage for a given order size in bps
    pub fn estimate_slippage_bps(&self, side: Side, quantity: Quantity) -> f64 {
        let levels = match side {
            Side::Buy => self
                .asks
                .iter()
                .map(|(k, v)| (k.price_int as f64 * self.tick_size, v.total_volume))
                .collect::<Vec<_>>(),
            Side::Sell => self
                .bids
                .iter()
                .rev()
                .map(|(k, v)| (k.price_int as f64 * self.tick_size, v.total_volume))
                .collect::<Vec<_>>(),
        };

        if levels.is_empty() {
            return 0.0;
        }

        let base_price = levels[0].0;
        let mut remaining = quantity;
        let mut weighted_price = 0.0;
        let mut filled_qty = 0.0;

        for (price, vol) in &levels {
            if remaining <= 0.0 {
                break;
            }
            let take = remaining.min(*vol);
            weighted_price += price * take;
            filled_qty += take;
            remaining -= take;
        }

        if filled_qty == 0.0 {
            return 0.0;
        }

        let avg_price = weighted_price / filled_qty;
        ((avg_price - base_price).abs() / base_price) * 10000.0
    }

    // --- Private ---

    fn key(price: Price, tick_size: Price) -> OrderKey {
        OrderKey::new(price, tick_size)
    }

    fn add_to_book(&mut self, order: &Order) {
        let price = order.price.unwrap();
        let key = Self::key(price, self.tick_size);

        let book = match order.side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        book.entry(key).or_default().total_volume += order.remaining();

        book.entry(key).or_default().orders.push(order.clone());
    }

    #[allow(unused_assignments)]
    fn match_order(&mut self, mut order: Order) -> QuantResult<Vec<Fill>> {
        let mut fills = Vec::new();
        let opposing = match order.side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };

        while order.remaining() > 0.0 && !opposing.is_empty() {
            // Extract best price from the key before mutable operations
            let best_price = {
                let first = opposing.first_entry().unwrap();
                first.key().price_int as f64 * self.tick_size
            };
            let mut first = opposing.first_entry().unwrap();
            let level = first.get_mut();

            // Match against orders at this level
            let mut to_remove = Vec::new();
            for (i, resting) in level.orders.iter_mut().enumerate() {
                if order.remaining() <= 0.0 {
                    break;
                }
                let fill_qty = order.remaining().min(resting.remaining());
                let fill_price = best_price;

                resting.filled += fill_qty;
                order.filled += fill_qty;
                level.total_volume -= fill_qty;

                if resting.is_filled() {
                    resting.status = OrderStatus::Filled;
                    to_remove.push(i);
                } else {
                    resting.status = OrderStatus::PartiallyFilled;
                }

                fills.push(Fill {
                    order_id: order.id,
                    side: order.side,
                    price: fill_price,
                    quantity: fill_qty,
                    timestamp: chrono::Utc::now().naive_utc(),
                    slippage_pct: if order.order_type == OrderType::Market {
                        let ref_price = self.last_price.max(0.01);
                        ((fill_price - ref_price).abs() / ref_price) * 100.0
                    } else {
                        0.0
                    },
                });
            }

            // Remove filled orders (in reverse to preserve indices)
            for i in to_remove.into_iter().rev() {
                level.orders.remove(i);
            }

            // Remove empty levels
            if level.total_volume <= 0.0 && level.orders.is_empty() {
                opposing.pop_first();
            }
        }

        if order.filled > 0.0 {
            self.last_price = fills.last().map(|f| f.price).unwrap_or(self.last_price);
            // Update incoming order status (caller may inspect)
            order.status = if order.is_filled() {
                OrderStatus::Filled
            } else {
                OrderStatus::PartiallyFilled
            };
        } else {
            order.status = OrderStatus::Rejected;
        }

        self.fills.extend(fills.clone());
        self.update_estimated_spread();

        Ok(fills)
    }

    fn update_estimated_spread(&mut self) {
        if let (Some(bid), Some(ask)) = (self.best_bid(), self.best_ask()) {
            self.estimated_spread = ask - bid;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_order_placement() {
        let mut ob = OrderBook::new("XAUUSD");
        ob.submit_limit(Side::Buy, 2650.0, 1.0).unwrap();
        ob.submit_limit(Side::Sell, 2655.0, 1.0).unwrap();

        assert_eq!(ob.best_bid(), Some(2650.0));
        assert_eq!(ob.best_ask(), Some(2655.0));
        assert_eq!(ob.mid_price(), Some(2652.5));
    }

    #[test]
    fn test_market_order_match() {
        let mut ob = OrderBook::new("TEST");
        ob.submit_limit(Side::Sell, 100.0, 2.0).unwrap();

        let fills = ob.submit_market(Side::Buy, 1.0).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].price, 100.0);
        assert_eq!(fills[0].quantity, 1.0);
    }

    #[test]
    fn test_partial_fill() {
        let mut ob = OrderBook::new("TEST");
        ob.submit_limit(Side::Sell, 100.0, 1.0).unwrap();

        let fills = ob.submit_market(Side::Buy, 3.0).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].quantity, 1.0); // only 1.0 available
    }
}
