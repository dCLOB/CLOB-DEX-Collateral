use crate::error::Error;
use crate::order::{AddField, NewAccountOrder, Order};
use soroban_sdk::{contracttype, Env};

#[derive(Clone, Debug)]
#[contracttype]
pub struct PriceStore {
    pub price: u128,
    orders_id_counter: u64,
    pub orders: soroban_sdk::Vec<Option<Order>>,
    order_ids: soroban_sdk::Map<u64, u32>,
}

impl PriceStore {
    pub fn new(price: u128, env: &Env) -> Self {
        Self {
            price,
            orders_id_counter: u64::default(),
            orders: soroban_sdk::vec![env],
            order_ids: soroban_sdk::map![env],
        }
    }

    pub fn is_empty(&self) -> bool {
        assert!(
            (self.order_ids.is_empty() && self.orders.iter().all(|el| el.is_none()))
                || (!self.order_ids.is_empty() && self.orders.iter().any(|el| el.is_some())),
            "Inconsistent state of the price node"
        );

        self.order_ids.is_empty()
    }

    pub fn remove_order(&mut self, order_id: u64) -> Option<Order> {
        let index = self.order_ids.get(order_id)?;

        let removed_order = self.orders.get(index).flatten();

        self.order_ids.remove(order_id);
        self.orders.set(index, None);

        removed_order
    }

    pub fn add_order(&mut self, order: NewAccountOrder) -> u64 {
        self.orders_id_counter += 1;
        let id = self.orders_id_counter;

        self.orders.push_back(Some(order.into_order(id)));
        self.order_ids.set(id, self.orders.len() - 1);

        id
    }

    pub fn update_order(&mut self, key: u64, order: Order) -> Option<()> {
        let index = self.order_ids.get(key)?;

        self.orders.set(index, Some(order));

        Some(())
    }

    pub fn try_get(&self, order_id: u64) -> Result<Order, Error> {
        let index = self
            .order_ids
            .get(order_id)
            .ok_or(Error::PriceStoreInvalidIndex)?;

        self.orders
            .get(index)
            .flatten()
            .ok_or(Error::PriceStoreOrderNotFoundByIndex)
    }

    pub fn iter(&self) -> impl Iterator<Item = Order> {
        self.orders.iter().filter_map(|opt| opt)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::order::NewAccountOrder;
    use crate::price_store::PriceStore;
    use soroban_sdk::Env;
    use soroban_sdk::{testutils::Address as AddressTestTrait, Address};
    #[test]
    fn test_create_price_store() {
        let env: Env = soroban_sdk::Env::default();
        let price_store = PriceStore::new(100, &env);

        assert_eq!(price_store.price, 100);
        assert!(price_store.orders.is_empty());
        assert!(price_store.order_ids.is_empty());
        assert!(price_store.is_empty());
    }

    #[test]
    fn test_add_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut price_store = PriceStore::new(100, &env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = price_store.add_order(order.clone());

        assert_eq!(order_id, 1);
        assert!(!price_store.is_empty());
        assert_eq!(price_store.orders.len(), 1);
        assert_eq!(price_store.order_ids.len(), 1);

        let retrieved_order = price_store.try_get(order_id);
        assert!(retrieved_order.is_ok());
        assert_eq!(retrieved_order.unwrap().quantity, 50);
    }

    #[test]
    fn test_remove_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut price_store = PriceStore::new(100, &env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = price_store.add_order(order.clone());

        let removed_order = price_store.remove_order(order_id);

        assert!(removed_order.is_some());
        assert_eq!(removed_order.unwrap().quantity, 50);

        assert!(price_store.try_get(order_id).is_err());
        assert!(price_store.is_empty());
    }

    #[test]
    fn test_remove_order_with_multiple_orders() {
        let env: Env = soroban_sdk::Env::default();
        let mut price_store = PriceStore::new(100, &env);

        let order1 = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order2 = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order1_id = price_store.add_order(order1.clone());
        let order2_id = price_store.add_order(order2.clone());

        let removed_order = price_store.remove_order(order1_id);

        assert!(removed_order.is_some());
        assert_eq!(removed_order.unwrap().quantity, 50);

        assert!(price_store.try_get(order1_id).is_err());
        assert!(!price_store.is_empty());

        let remaining_order = price_store.try_get(order2_id);
        assert!(remaining_order.is_ok());
        assert_eq!(remaining_order.unwrap().quantity, 30);
    }

    #[test]
    fn test_update_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut price_store = PriceStore::new(100, &env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = price_store.add_order(order.clone());

        let mut updated_order = price_store.try_get(order_id).unwrap();
        updated_order.quantity = 100;

        let update_result = price_store.update_order(order_id, updated_order.clone());

        assert!(update_result.is_some());

        let retrieved_order = price_store.try_get(order_id);
        assert!(retrieved_order.is_ok());
        assert_eq!(retrieved_order.unwrap().quantity, 100);
    }

    #[test]
    fn test_iter() {
        let env: Env = soroban_sdk::Env::default();
        let mut price_store = PriceStore::new(100, &env);

        let order1 = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order2 = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        price_store.add_order(order1.clone());
        price_store.add_order(order2.clone());

        let mut iter = price_store.iter();

        let first_order = iter.next();
        let second_order = iter.next();
        let third_order = iter.next();

        assert!(first_order.is_some());
        assert_eq!(first_order.unwrap().quantity, 50);

        assert!(second_order.is_some());
        assert_eq!(second_order.unwrap().quantity, 30);

        assert!(third_order.is_none());
    }
}
