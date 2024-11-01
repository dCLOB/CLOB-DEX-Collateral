use soroban_sdk::{contracttype, Env};

use crate::{
    order::Order,
    order_statistic_tree::node::{Key, NodeId},
};

#[derive(Clone)]
#[contracttype]
pub struct PriceStore {
    pub price: NodeId,
    orders_id_counter: Key,
    orders: soroban_sdk::Vec<Order>,
    order_ids: soroban_sdk::Map<Key, u32>,
}

impl core::borrow::Borrow<NodeId> for PriceStore {
    fn borrow(&self) -> &NodeId {
        &self.price
    }
}

impl PriceStore {
    pub fn new(price: NodeId, env: &Env) -> Self {
        Self {
            price,
            orders_id_counter: Key::default(),
            orders: soroban_sdk::vec![env],
            order_ids: soroban_sdk::map![env],
        }
    }

    pub fn is_empty(&self) -> bool {
        assert!(
            (self.orders.is_empty() && self.orders.is_empty())
                || (!self.orders.is_empty() && !self.orders.is_empty()),
            "Inconsistent state of the price node"
        );

        self.order_ids.is_empty()
    }

    pub fn remove_order(&mut self, order_id: Key) -> Option<Order> {
        let index = self.order_ids.get(order_id)?;

        let removed_order = self.orders.get(index);

        self.orders.remove(index);
        self.order_ids.remove(order_id);

        removed_order
    }

    pub fn add_order(&mut self, mut order: Order) -> Key {
        self.orders_id_counter += 1;
        let id = self.orders_id_counter;

        order.order_id = id;
        self.orders.push_back(order);
        self.order_ids.set(id, self.orders.len() - 1);

        id
    }

    pub fn update_order(&mut self, key: Key, order: Order) -> Option<()> {
        let index = self.order_ids.get(key)?;

        self.orders.set(index, order);

        Some(())
    }

    pub fn try_get(&self, order_id: Key) -> Option<Order> {
        let index = self.order_ids.get(order_id)?;

        self.orders.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = Order> {
        self.orders.iter()
    }
}
