use soroban_sdk::{contracttype, Env};

use crate::{
    error::Error,
    order::Order,
    order_statistic_tree::node::{Key, NodeId},
    orderbook::PriceLevelId,
    price_store::PriceStore,
};

// Store PriceLevelNode in direct sorted order based on price
#[contracttype]
pub struct PriceLevelStore {
    pub levels: soroban_sdk::Vec<PriceStore>,
    pub level_ids: soroban_sdk::Map<NodeId, u32>,
}

impl PriceLevelStore {
    pub fn new(env: &Env) -> Self {
        PriceLevelStore {
            levels: soroban_sdk::vec![env],
            level_ids: soroban_sdk::map![env],
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

    pub fn push_order(&mut self, order: Order, env: &Env) -> Key {
        let index = self.level_ids.get(order.price);

        match index {
            Some(index) => {
                let mut price_node = self.levels.get(index).unwrap();
                let order_id = price_node.add_order(order);
                self.levels.set(index, price_node);

                order_id
            }
            None => {
                let price_level = order.price;
                let mut price_node = PriceStore::new(price_level, env);
                let insertable_index = self.levels.binary_search(price_node.clone());

                if let Err(index) = insertable_index {
                    let order_id = price_node.add_order(order);
                    self.levels.insert(index, price_node);
                    self.level_ids.set(price_level, self.levels.len() - 1);
                    order_id
                } else {
                    soroban_sdk::panic_with_error!(env, Error::IncorrectPriceLevelStorageState)
                }
            }
        }
    }

    pub fn update_order(&mut self, price_id: PriceLevelId, order: Order) -> Option<()> {
        let PriceLevelId { id, price } = price_id;

        let index = self.level_ids.get(price)?;

        let mut price_level = self.levels.get(index)?;

        price_level.update_order(id, order);

        Some(())
    }

    pub fn remove_order(&mut self, price: NodeId, order_id: Key) -> Option<Order> {
        let index = self.level_ids.get(price)?;

        let mut price_level_node = self.levels.get(index)?;

        let order = price_level_node.remove_order(order_id)?;

        if price_level_node.is_empty() {
            self.levels.remove(index);
            self.level_ids.remove(price);
        } else {
            self.levels.set(index, price_level_node);
        }

        Some(order)
    }

    pub fn try_get(&self, price: NodeId, order_id: Key) -> Option<Order> {
        let index = self.level_ids.get(price)?;

        let price_level_node = self.levels.get(index)?;

        price_level_node.try_get(order_id)
    }
}
