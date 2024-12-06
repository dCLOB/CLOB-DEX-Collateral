#[cfg(test)]
use rand::Rng;
use soroban_sdk::Env;

use crate::{
    node_impl::{NodeColor, NodeView},
    order_statistic_tree::{
        node::{NodeViewHolder, NodeViewInterface, StorageAccessor},
        tree::OrderStatisticTree,
    },
};

extern crate std;

macro_rules! orderbook_scope {
    ($env:expr, $code:block) => {{
        let id = $env.register_contract(None, crate::Contract {});
        $env.as_contract(&id, || $code);
    }};
}

#[test]
fn test_insert_to_right_subtree_and_retrieve_node() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        tree.insert(10, 1).unwrap();
        tree.insert(20, 2).unwrap();
        tree.insert(15, 3).unwrap();

        assert!(tree.exists(10));
        assert!(tree.exists(20));
        assert!(tree.exists(15));

        let node_10 = tree
            .storage_accessor
            .to_node_holder(NodeView::new(10))
            .load()
            .unwrap();

        assert_eq!(node_10.parent().to_raw(), Some(15));
        assert_eq!(node_10.left().to_raw(), None);
        assert_eq!(node_10.right().to_raw(), None);
        assert_eq!(node_10.color(), NodeColor::Red);

        let node_20 = tree
            .storage_accessor
            .to_node_holder(NodeView::new(20))
            .load()
            .unwrap();
        assert_eq!(node_20.parent().to_raw(), Some(15));
        assert_eq!(node_20.left().to_raw(), None);
        assert_eq!(node_20.right().to_raw(), None);
        assert_eq!(node_20.color(), NodeColor::Red);

        let node_15 = tree
            .storage_accessor
            .to_node_holder(NodeView::new(15))
            .load()
            .unwrap();
        assert_eq!(node_15.parent().to_raw(), None);
        assert_eq!(node_15.left().to_raw(), Some(10));
        assert_eq!(node_15.right().to_raw(), Some(20));
        assert_eq!(node_15.color(), NodeColor::Black);

        assert_eq!(tree.root.get_view().to_raw(), Some(15));
    })
}

#[test]
fn test_insert_to_left_subtree_and_retrieve_node() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        tree.insert(20, 1).unwrap();
        tree.insert(10, 2).unwrap();
        tree.insert(15, 3).unwrap();

        assert!(tree.exists(10));
        assert!(tree.exists(20));
        assert!(tree.exists(15));

        let node_10 = tree
            .storage_accessor
            .to_node_holder(NodeView::new(10))
            .load()
            .unwrap();

        assert_eq!(node_10.parent().to_raw(), Some(15));
        assert_eq!(node_10.left().to_raw(), None);
        assert_eq!(node_10.right().to_raw(), None);
        assert_eq!(node_10.color(), NodeColor::Red);

        let node_20 = tree
            .storage_accessor
            .to_node_holder(NodeView::new(20))
            .load()
            .unwrap();
        assert_eq!(node_20.parent().to_raw(), Some(15));
        assert_eq!(node_20.left().to_raw(), None);
        assert_eq!(node_20.right().to_raw(), None);
        assert_eq!(node_20.color(), NodeColor::Red);

        let node_15 = tree
            .storage_accessor
            .to_node_holder(NodeView::new(15))
            .load()
            .unwrap();
        assert_eq!(node_15.parent().to_raw(), None);
        assert_eq!(node_15.left().to_raw(), Some(10));
        assert_eq!(node_15.right().to_raw(), Some(20));
        assert_eq!(node_15.color(), NodeColor::Black);

        assert_eq!(tree.root.get_view().to_raw(), Some(15));
    })
}

#[test]
fn add_1_value_and_test_for_first_and_last_values1() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);
        tree.insert(1, 1).unwrap();
        assert_eq!(tree.first().unwrap().to_raw(), Some(1));
        assert_eq!(tree.last().unwrap().to_raw(), Some(1));
    })
}

#[test]
fn add_10_values_and_test_for_first_and_last_values1() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);
        for i in 1..=10 {
            tree.insert(i, (i + 1) as u64).unwrap();
        }
        assert!(!tree.exists(0));
        assert!(tree.exists(1));
        assert!(tree.exists(5));
        assert!(tree.exists(10));
        assert_eq!(tree.first().unwrap().to_raw(), Some(1));
        assert_eq!(tree.last().unwrap().to_raw(), Some(10));

        verify_tree(&tree).unwrap();
    })
}

#[test]
fn remove_black_node_in_the_middle_with_one_left_leaf_child1() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        for i in 1..=10 {
            tree.insert(i, (i + 1) as u64).unwrap();
        }
        for i in 1..=10 {
            assert!(tree.exists(i));
        }

        tree.remove(8, 9).unwrap();
        assert!(!tree.exists(8));
        assert_eq!(Some(1), tree.first().unwrap().to_raw());
        assert_eq!(Some(10), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();
    })
}

#[test]
fn remove_black_node_in_the_middle_with_one_left_child1() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        for i in 1..=12 {
            tree.insert(i, (i + 1) as u64).unwrap();
        }
        for i in 1..=12 {
            assert!(tree.exists(i));
        }

        tree.remove(8, 9).unwrap();

        assert!(!tree.exists(8));
        assert_eq!(Some(1), tree.first().unwrap().to_raw());
        assert_eq!(Some(12), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();
    })
}

#[test]
fn add_and_remove_test1() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        for i in 1..=10 {
            tree.insert(i, (i + 1) as u64).unwrap();
        }
        for i in 1..=10 {
            assert!(tree.exists(i));
        }

        verify_tree(&tree).unwrap();

        tree.remove(1, 2).unwrap();
        assert!(!tree.exists(1));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(10), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(5, 6).unwrap();
        assert!(!tree.exists(5));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(10), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(10, 11).unwrap();
        assert!(!tree.exists(10));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(3, 4).unwrap();
        assert!(!tree.exists(3));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(7, 8).unwrap();
        assert!(!tree.exists(7));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(4, 5).unwrap();
        assert!(!tree.exists(4));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(8, 9).unwrap();
        assert!(!tree.exists(8));
        assert_eq!(Some(2), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(2, 3).unwrap();
        assert!(!tree.exists(2));
        assert_eq!(Some(6), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(6, 7).unwrap();
        assert!(!tree.exists(6));
        assert_eq!(Some(9), tree.first().unwrap().to_raw());
        assert_eq!(Some(9), tree.last().unwrap().to_raw());
        verify_tree(&tree).unwrap();

        tree.remove(9, 10).unwrap();
        assert!(!tree.exists(9));
        assert_eq!(None, tree.first().unwrap().to_raw());
        assert_eq!(None, tree.last().unwrap().to_raw());
    });
}

#[test]
fn tree_with_100_entries_is_valid1() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);
        let mut rng = rand::thread_rng();

        for _ in 1..=100 {
            let value: u128 = rng.gen_range(0..5000);
            tree.insert(value, (value + 1) as u64).unwrap();
        }
        verify_tree(&tree).unwrap();
    })
}

#[test]
fn tree_with_100_entries_is_valid_asc() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        for value in 1..=100 {
            tree.insert(value, (value + 1) as u64).unwrap();
        }
        verify_tree(&tree).unwrap();
    })
}

#[test]
fn tree_with_100_entries_is_valid_desc() {
    let env: Env = soroban_sdk::Env::default();
    orderbook_scope!(env, {
        let mut tree = OrderStatisticTree::new(&env);

        for value in (1..=100).rev() {
            tree.insert(value, (value - 1) as u64).unwrap();
        }
        verify_tree(&tree).unwrap();
    })
}

fn verify_tree(tree: &OrderStatisticTree<&Env>) -> Result<(), std::string::String> {
    verify_tree_property_1(tree.root)?;
    verify_tree_property_2(tree)?;
    verify_tree_property_4(tree.root)?;
    verify_tree_property_5(tree.root)?;
    Ok(())
}

fn verify_tree_property_1(node: NodeViewHolder<&Env>) -> Result<(), std::string::String> {
    if !node.is_empty() {
        let node = node.load().unwrap();
        if node.color() == NodeColor::Red || node.color() == NodeColor::Black {
            verify_tree_property_1(node.left())?;
            verify_tree_property_1(node.right())?;
        } else {
            return Err(std::format!(
                "Node color must be black or red, {:?}",
                *node.node_view()
            ));
        }
    }
    Ok(())
}

fn verify_tree_property_2(tree: &OrderStatisticTree<&Env>) -> Result<(), std::string::String> {
    if !tree.root.is_empty() {
        let root_node = tree.root.load().unwrap();
        if root_node.color() == NodeColor::Red {
            return Err("OrderStatisticTree root node color must be black.".into());
        }
    }
    Ok(())
}

fn verify_tree_property_4(node: NodeViewHolder<&Env>) -> Result<(), std::string::String> {
    if !node.is_empty() {
        let node = node.load().unwrap();
        let node_left = node.left();
        let node_right = node.right();

        if node.color() == NodeColor::Red
            && (!node_left.is_empty() && node_left.load().unwrap().color() == NodeColor::Red
                || !node_right.is_empty() && node_right.load().unwrap().color() == NodeColor::Red)
        {
            return Err(std::format!(
                "Red parent node has one or more red child nodes, {:?}",
                *node.node_view()
            ));
        }
        verify_tree_property_4(node.left())?;
        verify_tree_property_4(node.right())?;
    }
    Ok(())
}

fn verify_tree_property_5(node: NodeViewHolder<&Env>) -> Result<(), std::string::String> {
    let path_black_count_to_min = path_black_count_to_min_node(node);
    let path_black_count_to_max = path_black_count_to_max_node(node);
    if path_black_count_to_min != path_black_count_to_max {
        return Err(std::format!(
            "Path black count to first {} does not match path black count to last {}",
            path_black_count_to_min,
            path_black_count_to_max
        ));
    } else {
        verify_tree_property_5_rec(node, path_black_count_to_max, 0)?;
    }
    Ok(())
}

fn verify_tree_property_5_rec(
    node: NodeViewHolder<&Env>,
    black_count: usize,
    path_black_count: usize,
) -> Result<(), std::string::String> {
    if node.is_empty() && black_count != path_black_count {
        return Err(std::format!(
            "Patch black count expected {}, patch black count found {}.",
            black_count,
            path_black_count
        ));
    } else if !node.is_empty() {
        let cur_node = node.load().unwrap();
        let new_path_black_count = if cur_node.color() == NodeColor::Black {
            path_black_count + 1
        } else {
            path_black_count
        };
        verify_tree_property_5_rec(cur_node.left(), black_count, new_path_black_count)?;
        verify_tree_property_5_rec(cur_node.right(), black_count, new_path_black_count)?;
    }
    Ok(())
}

fn path_black_count_to_min_node(node: NodeViewHolder<&Env>) -> usize {
    let mut black_count = 0;
    let mut current = node;
    while !current.is_empty() {
        let node = current.load().unwrap();
        if node.color() == NodeColor::Black {
            black_count += 1;
        }
        current = node.left();
    }
    black_count
}

fn path_black_count_to_max_node(node: NodeViewHolder<&Env>) -> usize {
    let mut black_count = 0;
    let mut current = node;
    while !current.is_empty() {
        let node = current.load().unwrap();
        if node.color() == NodeColor::Black {
            black_count += 1;
        }
        current = node.right();
    }
    black_count
}
