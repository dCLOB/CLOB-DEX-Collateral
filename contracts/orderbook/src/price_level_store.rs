use crate::{
    error::Error,
    order::{NewAccountOrder, Order},
    orderbook::PriceLevelId,
    price_store::PriceStore,
};
use soroban_sdk::{contracttype, Env, Vec};

#[contracttype]
pub struct PriceLevelStore {
    pub levels: soroban_sdk::Vec<PriceStore>,
    pub levels_price: soroban_sdk::Vec<u128>,
}

impl PriceLevelStore {
    pub fn new(env: &Env) -> Self {
        PriceLevelStore {
            levels: soroban_sdk::vec![env],
            levels_price: soroban_sdk::vec![env],
        }
    }

    // Returns an iterator over the PriceLevel in the [`MultiplePriceLevels`] in DESC order.
    pub(crate) fn iter_levels_rev(&self) -> impl Iterator<Item = PriceStore> {
        self.levels.iter().rev()
    }

    // Returns an iterator over the PriceLevel in the [`MultiplePriceLevels`] in ASC order.
    pub(crate) fn iter_levels(&self) -> impl Iterator<Item = PriceStore> {
        self.levels.iter()
    }

    pub fn push_order(&mut self, order: NewAccountOrder, env: &Env) -> u64 {
        let index = self.levels_price.binary_search(order.price);

        match index {
            Ok(index) => {
                let mut price_node = self.levels.get(index).unwrap();
                let order_id = price_node.add_order(order);
                self.levels.set(index, price_node);

                order_id
            }
            Err(index) => {
                let price_level = order.price;
                let mut price_node = PriceStore::new(price_level, env);

                let order_id = price_node.add_order(order);
                self.levels_price.insert(index, price_level);
                self.levels.insert(index, price_node);
                order_id
            }
        }
    }

    pub fn update_order(&mut self, price_id: PriceLevelId, order: Order) -> Option<()> {
        let PriceLevelId { id, price } = price_id;

        let index = self.levels_price.binary_search(price).ok()?;

        let mut price_level = self.levels.get(index)?;

        price_level.update_order(id, order);

        self.levels.set(index, price_level);

        Some(())
    }

    pub fn remove_order(&mut self, price: u128, order_id: u64) -> Result<Order, Error> {
        let index = self
            .levels_price
            .binary_search(price)
            .map_err(|_| Error::LevelsStorePriceNotFound)?;

        let mut price_level_node = self
            .levels
            .get(index)
            .ok_or(Error::LevelsStoreLevelNotFound)?;

        let order = price_level_node
            .remove_order(order_id)
            .ok_or(Error::LevelsStoreRemoveFailed)?;

        if price_level_node.is_empty() {
            self.levels.remove(index);
            self.levels_price.remove(index);
        } else {
            self.levels.set(index, price_level_node);
        }

        Ok(order)
    }

    pub fn try_get(&self, price: u128, order_id: u64) -> Result<Order, Error> {
        let index = self
            .levels_price
            .binary_search(price)
            .map_err(|_| Error::LevelsStorePriceNotFound)?;

        let price_level_node = self
            .levels
            .get(index)
            .ok_or(Error::LevelsStoreLevelNotFound)?;

        price_level_node.try_get(order_id)
    }

    pub fn get_orders(&self, price: u128, env: &Env) -> Vec<Order> {
        let index = self.levels_price.binary_search(price).unwrap();

        let price_level_node = self.levels.get(index).unwrap();

        let mut orders = Vec::new(&env);

        price_level_node
            .orders
            .iter()
            .filter_map(|el| el)
            .for_each(|el| orders.push_back(el));

        orders
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::order::{NewAccountOrder, Order};
    use crate::orderbook::PriceLevelId;
    use crate::price_level_store::PriceLevelStore;
    use soroban_sdk::Env;
    use soroban_sdk::{testutils::Address as AddressTestTrait, Address};

    #[test]
    fn test_price_level_store_initialization() {
        let env: Env = soroban_sdk::Env::default();

        let store = PriceLevelStore::new(&env);

        assert!(store.levels.is_empty());
    }

    #[test]
    fn test_push_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = store.push_order(order.clone(), &env);

        assert_eq!(store.levels.len(), 1);
        assert!(store.levels_price.binary_search(100).is_ok());
        assert_eq!(order_id, 1);

        // Verify that the order is stored correctly
        let stored_order = store.try_get(order.price, order_id).unwrap();
        assert_eq!(stored_order.price, order.price);
        assert_eq!(stored_order.quantity, order.quantity);
    }

    #[test]
    fn test_push_multiple_orders_same_price() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

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

        let order_id1 = store.push_order(order1.clone(), &env);
        let order_id2 = store.push_order(order2.clone(), &env);

        assert_eq!(store.levels.len(), 1);
        assert!(store.levels_price.binary_search(100).is_ok());
        assert_ne!(order_id1, order_id2);

        // Verify that both orders are stored
        let stored_order1 = store.try_get(order1.price, order_id1).unwrap();
        let stored_order2 = store.try_get(order2.price, order_id2).unwrap();

        assert_eq!(stored_order1.quantity, 50);
        assert_eq!(stored_order2.quantity, 30);
    }

    #[test]
    fn test_push_orders_different_prices() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let order1 = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order2 = NewAccountOrder {
            account: Address::generate(&env),
            price: 110,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        store.push_order(order1.clone(), &env);
        store.push_order(order2.clone(), &env);

        assert_eq!(store.levels.len(), 2);
        assert!(store.levels_price.binary_search(100).is_ok());
        assert!(store.levels_price.binary_search(110).is_ok());

        let stored_order1 = store.try_get(order1.price, 1).unwrap();
        let stored_order2 = store.try_get(order2.price, 1).unwrap();

        assert_eq!(stored_order1.quantity, 50);
        assert_eq!(stored_order2.quantity, 30);
    }

    #[test]
    fn test_update_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let account = Address::generate(&env);
        let order = NewAccountOrder {
            account: account.clone(),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = store.push_order(order.clone(), &env);

        let updated_order = Order {
            price: 100,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
            order_id,
            account,
        };

        let price_id = PriceLevelId {
            id: order_id,
            price: 100,
        };

        let result = store.update_order(price_id, updated_order.clone());
        assert!(result.is_some());

        // Verify the order is updated
        let stored_order = store.try_get(100, order_id).unwrap();
        assert_eq!(stored_order.quantity, 30);
    }

    #[test]
    fn test_remove_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = store.push_order(order.clone(), &env);

        let removed_order = store.remove_order(100, order_id);

        assert!(removed_order.is_ok());
        assert_eq!(removed_order.unwrap().quantity, 50);

        // Verify the level is empty
        assert!(store.levels.is_empty());
        assert!(store.levels_price.is_empty());
    }

    #[test]
    fn test_remove_order_with_multiple_orders() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

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

        let order1_id = store.push_order(order1.clone(), &env);
        let order2_id = store.push_order(order2.clone(), &env);

        let removed_order = store.remove_order(100, order1_id);

        assert!(removed_order.is_ok());
        assert_eq!(removed_order.unwrap().quantity, 50);

        assert!(store.try_get(100, order1_id).is_err());

        let remaining_order = store.try_get(100, order2_id);
        assert!(remaining_order.is_ok());
        assert_eq!(remaining_order.unwrap().quantity, 30);
    }

    #[test]
    fn test_remove_order_with_price_node_deletion() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let order1 = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order2 = NewAccountOrder {
            account: Address::generate(&env),
            price: 200,
            quantity: 30,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order1_id = store.push_order(order1.clone(), &env);
        let order2_id = store.push_order(order2.clone(), &env);

        let removed_order = store.remove_order(100, order1_id);

        assert!(removed_order.is_ok());
        assert_eq!(removed_order.unwrap().quantity, 50);

        assert!(store.try_get(100, order1_id).is_err());
        assert!(store.levels_price.binary_search(100).is_err());

        let remaining_order = store.try_get(200, order2_id);
        assert!(remaining_order.is_ok());
        assert_eq!(remaining_order.unwrap().quantity, 30);

        assert!(store.levels_price.binary_search(200).is_ok());
    }

    #[test]
    fn test_iterate_levels() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let orders = std::vec![
            NewAccountOrder {
                account: Address::generate(&env),
                price: 100,
                quantity: 50,
                fee_amount: 0,
                fee_token_asset: Address::generate(&env),
            },
            NewAccountOrder {
                account: Address::generate(&env),
                price: 110,
                quantity: 30,
                fee_amount: 0,
                fee_token_asset: Address::generate(&env),
            },
        ];

        for order in orders.clone() {
            store.push_order(order, &env);
        }

        // Test ascending order
        let mut iter = store.iter_levels();
        let first_level = iter.next().unwrap();
        let second_level = iter.next().unwrap();

        assert_eq!(first_level.price, 100);
        assert_eq!(second_level.price, 110);

        // Test descending order
        let mut iter_rev = store.iter_levels_rev();
        let first_level_rev = iter_rev.next().unwrap();
        let second_level_rev = iter_rev.next().unwrap();

        assert_eq!(first_level_rev.price, 110);
        assert_eq!(second_level_rev.price, 100);
    }

    #[test]
    fn test_remove_last_order_at_price() {
        let env: Env = soroban_sdk::Env::default();
        let mut store = PriceLevelStore::new(&env);

        let order = NewAccountOrder {
            account: Address::generate(&env),
            price: 100,
            quantity: 50,
            fee_amount: 0,
            fee_token_asset: Address::generate(&env),
        };

        let order_id = store.push_order(order.clone(), &env);

        let _ = store.remove_order(100, order_id);

        assert!(store.levels.is_empty());
        assert!(store.levels_price.is_empty());
    }

    #[test]
    fn test_add_orders_in_price_decrease_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut price_level_store = PriceLevelStore::new(&env);

        let account = Address::generate(&env);

        let order_1 = NewAccountOrder {
            account: account.clone(),
            quantity: 50,
            price: 100,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let order_2 = NewAccountOrder {
            account,
            quantity: 70,
            price: 95,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        price_level_store.push_order(order_1.clone(), &env);
        price_level_store.push_order(order_2.clone(), &env);

        let order = price_level_store.try_get(95, 1).unwrap();

        assert_eq!(order.quantity, order_2.quantity);

        let order = price_level_store.try_get(100, 1).unwrap();

        assert_eq!(order.quantity, order_1.quantity);
    }
}
