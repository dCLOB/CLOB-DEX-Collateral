use crate::{
    error::Error,
    order::{NewAccountOrder, Order, OrderSide},
    price_level_store::PriceLevelStore,
    price_store::PriceStore,
};
use soroban_sdk::{contracttype, Env};

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
    pub id: u64,
    pub price: u128,
}

#[contracttype]
pub struct OrderBook {
    pub(crate) buy_orders: PriceLevelStore,
    pub(crate) sell_orders: PriceLevelStore,
}

impl OrderBook {
    pub fn new(env: &Env) -> Self {
        Self {
            buy_orders: PriceLevelStore::new(env),
            sell_orders: PriceLevelStore::new(env),
        }
    }

    pub fn add_buy_order(&mut self, order: NewAccountOrder, env: &Env) -> OrderBookId {
        let price = order.price;
        let key = self.buy_orders.push_order(order, env);

        OrderBookId::buy_id(price, key)
    }

    pub fn add_sell_order(&mut self, order: NewAccountOrder, env: &Env) -> OrderBookId {
        let price = order.price;
        let key = self.sell_orders.push_order(order, env);

        OrderBookId::sell_id(price, key)
    }

    pub fn try_get(&self, order_book_id: OrderBookId) -> Result<Order, Error> {
        match order_book_id {
            OrderBookId::BuyId(PriceLevelId { id, price }) => self.buy_orders.try_get(price, id),
            OrderBookId::SellId(PriceLevelId { id, price }) => self.sell_orders.try_get(price, id),
        }
    }

    pub fn remove_order(&mut self, order_book_id: OrderBookId) -> Result<Order, Error> {
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

    pub fn best_buy_price(&self) -> Option<u128> {
        self.maker_orders_iter(OrderSide::BUY)
            .next()
            .map(|(_, order)| order.price)
    }

    pub fn best_sell_price(&self) -> Option<u128> {
        self.maker_orders_iter(OrderSide::SELL)
            .next()
            .map(|(_, order)| order.price)
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

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::order::{NewAccountOrder, Order, OrderSide};
    use crate::orderbook::{OrderBook, OrderBookId};
    use soroban_sdk::Env;
    use soroban_sdk::{testutils::Address as AddressTestTrait, Address};

    #[test]
    fn test_order_book_initialization() {
        let env: Env = soroban_sdk::Env::default();
        let order_book = OrderBook::new(&env);

        assert!(order_book.buy_orders.levels.is_empty());
        assert!(order_book.sell_orders.levels.is_empty());
    }

    #[test]
    fn test_add_buy_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = order_book.add_buy_order(order.clone(), &env);

        assert!(matches!(order_id, OrderBookId::BuyId(_)));

        // Verify the order is stored
        let stored_order = order_book.try_get(order_id.clone()).unwrap();
        assert_eq!(stored_order.price, order.price);
        assert_eq!(stored_order.quantity, order.quantity);
    }

    #[test]
    fn test_add_sell_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = order_book.add_sell_order(order.clone(), &env);

        assert!(matches!(order_id, OrderBookId::SellId(_)));

        // Verify the order is stored
        let stored_order = order_book.try_get(order_id.clone()).unwrap();
        assert_eq!(stored_order.price, order.price);
        assert_eq!(stored_order.quantity, order.quantity);
    }

    #[test]
    fn test_try_get_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = order_book.add_buy_order(order.clone(), &env);

        let stored_order = order_book.try_get(order_id).unwrap();

        assert_eq!(stored_order.price, 100);
        assert_eq!(stored_order.quantity, 50);
    }

    #[test]
    fn test_remove_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = order_book.add_buy_order(order.clone(), &env);

        let removed_order = order_book.remove_order(order_id.clone());

        assert!(removed_order.is_ok());
        assert_eq!(removed_order.unwrap().quantity, 50);

        // Verify the order is removed
        assert!(order_book.try_get(order_id).is_err());
    }

    #[test]
    fn test_update_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);
        let account = Address::generate(&env);

        let order = NewAccountOrder {
            account: account.clone(),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = order_book.add_buy_order(order.clone(), &env);

        let updated_order = Order {
            price: 100,
            quantity: 30,
            order_id: 1,
            account,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let result = order_book.update_order(order_id.clone(), updated_order.clone());
        assert!(result.is_some());

        // Verify the order is updated
        let stored_order = order_book.try_get(order_id).unwrap();
        assert_eq!(stored_order.quantity, 30);
    }

    #[test]
    fn test_best_buy_and_sell_prices() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let buy_order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let sell_order = NewAccountOrder {
            account: Address::generate(&env),
            price: 110,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        order_book.add_buy_order(buy_order.clone(), &env);
        order_book.add_sell_order(sell_order.clone(), &env);

        assert_eq!(order_book.best_buy_price(), Some(100));
        assert_eq!(order_book.best_sell_price(), Some(110));
    }

    #[test]
    fn test_maker_orders_iter() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let buy_orders = std::vec![
            NewAccountOrder {
                account: Address::generate(&env),
                price: 100,
                quantity: 50,
                fee_amount: 0,
                fee_token_asset: Address::generate(&env),
            },
            NewAccountOrder {
                account: Address::generate(&env),
                price: 90,
                quantity: 20,
                fee_amount: 0,
                fee_token_asset: Address::generate(&env),
            },
        ];

        let sell_orders = std::vec![
            NewAccountOrder {
                account: Address::generate(&env),
                price: 110,
                quantity: 30,
                fee_amount: 0,
                fee_token_asset: Address::generate(&env),
            },
            NewAccountOrder {
                account: Address::generate(&env),
                price: 120,
                quantity: 40,
                fee_amount: 0,
                fee_token_asset: Address::generate(&env),
            },
        ];

        for order in buy_orders.clone() {
            order_book.add_buy_order(order, &env);
        }

        for order in sell_orders.clone() {
            order_book.add_sell_order(order, &env);
        }

        // Verify buy orders
        let buy_iter = order_book.maker_orders_iter(OrderSide::BUY);
        let buy_prices: std::vec::Vec<u128> = buy_iter.map(|(_, order)| order.price).collect();
        assert_eq!(buy_prices, std::vec![100, 90]);

        // Verify sell orders
        let sell_iter = order_book.maker_orders_iter(OrderSide::SELL);
        let sell_prices: std::vec::Vec<u128> = sell_iter.map(|(_, order)| order.price).collect();
        assert_eq!(sell_prices, std::vec![110, 120]);
    }
}
