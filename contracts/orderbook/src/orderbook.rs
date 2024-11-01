use soroban_sdk::{contracttype, Env};

use crate::{
    order::{Order, OrderSide},
    order_statistic_tree::node::{Key, NodeId},
    price_level_store::PriceLevelStore,
    price_store::PriceStore,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub enum OrderBookId {
    BuyId(PriceLevelId),
    SellId(PriceLevelId),
}

impl OrderBookId {
    pub fn buy_id(price: u128, id: u64) -> Self {
        Self::BuyId(PriceLevelId { id, price })
    }

    pub fn sell_id(price: u128, id: u64) -> Self {
        Self::SellId(PriceLevelId { id, price })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct PriceLevelId {
    pub id: Key,
    pub price: NodeId,
}

#[contracttype]
pub struct OrderBook {
    buy_orders: PriceLevelStore,
    sell_orders: PriceLevelStore,
}

impl OrderBook {
    pub fn new(env: &Env) -> Self {
        Self {
            buy_orders: PriceLevelStore::new(env),
            sell_orders: PriceLevelStore::new(env),
        }
    }

    pub fn add_buy_order(&mut self, order: Order, env: &Env) -> OrderBookId {
        let price = order.price;
        let key = self.buy_orders.push_order(order, env);

        OrderBookId::buy_id(price, key)
    }

    pub fn add_sell_order(&mut self, order: Order, env: &Env) -> OrderBookId {
        let price = order.price;
        let key = self.sell_orders.push_order(order, env);

        OrderBookId::buy_id(price, key)
    }

    pub fn try_get(&self, order_book_id: OrderBookId) -> Option<Order> {
        match order_book_id {
            OrderBookId::BuyId(PriceLevelId { id, price }) => self.buy_orders.try_get(price, id),
            OrderBookId::SellId(PriceLevelId { id, price }) => self.sell_orders.try_get(price, id),
        }
    }

    pub fn remove_order(&mut self, order_book_id: OrderBookId) -> Option<Order> {
        match order_book_id {
            OrderBookId::BuyId(PriceLevelId { id, price }) => {
                self.buy_orders.remove_order(price, id)
            }
            OrderBookId::SellId(PriceLevelId { id, price }) => {
                self.sell_orders.remove_order(price, id)
            }
        }
    }

    pub fn update_order(&mut self, order_id: OrderBookId, order: Order) -> Option<()> {
        match order_id {
            OrderBookId::BuyId(price_level_id) => {
                self.buy_orders.update_order(price_level_id, order)
            }
            OrderBookId::SellId(price_level_id) => {
                self.sell_orders.update_order(price_level_id, order)
            }
        }
    }

    pub fn maker_orders_iter(
        &self,
        maker_side: OrderSide,
    ) -> impl Iterator<Item = (OrderBookId, Order)> {
        enum Either<L, R> {
            Left(L),
            Right(R),
        }

        impl<L, R> Iterator for Either<L, R>
        where
            L: Iterator,
            R: Iterator<Item = L::Item>,
        {
            type Item = L::Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Either::Left(l) => l.next(),
                    Either::Right(r) => r.next(),
                }
            }
        }

        fn wrap_iter<I: Iterator<Item = PriceStore>>(
            side: OrderSide,
            iter: I,
        ) -> impl Iterator<Item = (OrderBookId, Order)> {
            iter.flat_map(move |level| {
                level.iter().map(move |order| {
                    let order_book_id = match side {
                        OrderSide::BUY => OrderBookId::buy_id(level.price, order.order_id),
                        OrderSide::SELL => OrderBookId::sell_id(level.price, order.order_id),
                    };
                    (order_book_id, order)
                })
            })
        }

        match maker_side {
            OrderSide::BUY => {
                Either::Left(wrap_iter(maker_side, self.buy_orders.iter_levels_rev()))
            }
            OrderSide::SELL => Either::Right(wrap_iter(maker_side, self.sell_orders.iter_levels())),
        }
    }
}
