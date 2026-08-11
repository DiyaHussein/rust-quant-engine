use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Price level in the order book
pub type Price = f64;
/// Quantity (volume) in base units
pub type Quantity = f64;
/// Unique order identifier
pub type OrderId = u64;

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// An order submitted to the engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Price>,
    pub stop_price: Option<Price>,
    pub quantity: Quantity,
    pub filled: Quantity,
    pub status: OrderStatus,
    pub timestamp: NaiveDateTime,
}

/// A single trade execution (fill)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: OrderId,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: NaiveDateTime,
    pub slippage_pct: f64,
}

/// OHLCV bar (candlestick)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub timestamp: NaiveDateTime,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: Quantity,
}

impl Order {
    /// Create a new limit order
    pub fn limit(
        id: OrderId,
        side: Side,
        price: Price,
        quantity: Quantity,
        ts: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            stop_price: None,
            quantity,
            filled: 0.0,
            status: OrderStatus::Pending,
            timestamp: ts,
        }
    }

    /// Create a new market order
    pub fn market(id: OrderId, side: Side, quantity: Quantity, ts: NaiveDateTime) -> Self {
        Self {
            id,
            side,
            order_type: OrderType::Market,
            price: None,
            stop_price: None,
            quantity,
            filled: 0.0,
            status: OrderStatus::Pending,
            timestamp: ts,
        }
    }

    /// Remaining quantity to fill
    pub fn remaining(&self) -> Quantity {
        (self.quantity - self.filled).max(0.0)
    }

    /// Whether the order is fully filled
    pub fn is_filled(&self) -> bool {
        self.filled >= self.quantity
    }

    /// Whether the order is still active (can receive fills)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Pending | OrderStatus::PartiallyFilled
        )
    }
}
