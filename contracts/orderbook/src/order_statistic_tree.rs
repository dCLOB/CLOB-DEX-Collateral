use core::ops::{Deref, DerefMut};
use soroban_sdk::{contracttype, panic_with_error, Env, Map};

use crate::error::Error;

struct NodeHolder<'a> {
    key: u64,
    node: Node,
    nodes_map: &'a mut Map<u64, Node>,
}

impl<'a> Deref for NodeHolder<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<'a> DerefMut for NodeHolder<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<'a> Drop for NodeHolder<'a> {
    fn drop(&mut self) {
        self.nodes_map.set(self.key, self.node.clone());
    }
}

trait TreeOps {
    fn get_left_of(&self, value: u64) -> Option<Node>;
    fn get_left_of_mut<'a>(&'a mut self, value: u64) -> Option<NodeHolder<'a>>;

    fn get_right_of(&self, value: u64) -> Option<Node>;
    fn get_right_of_mut<'a>(&'a mut self, value: u64) -> Option<NodeHolder<'a>>;
}

trait GetMut<'a> {
    fn get_mut(&'a mut self, key: u64) -> Option<NodeHolder<'a>>;
}

impl<'a> GetMut<'a> for Map<u64, Node> {
    fn get_mut(&'a mut self, key: u64) -> Option<NodeHolder<'a>> {
        self.get(key).map(|node| NodeHolder {
            key,
            node,
            nodes_map: self,
        })
    }
}

impl TreeOps for Map<u64, Node> {
    fn get_left_of(&self, value: u64) -> Option<Node> {
        let res = self.get(value)?;

        self.get(res.left?)
    }

    fn get_right_of(&self, value: u64) -> Option<Node> {
        let res = self.get(value)?;

        self.get(res.right?)
    }

    fn get_left_of_mut<'a>(&'a mut self, value: u64) -> Option<NodeHolder<'a>> {
        let res = self.get(value)?;

        self.get_mut(res.left?)
    }

    fn get_right_of_mut<'a>(&'a mut self, value: u64) -> Option<NodeHolder<'a>> {
        let res = self.get(value)?;

        self.get_mut(res.right?)
    }
}

#[contracttype]
#[derive(Clone)]

struct Node {
    parent: Option<u64>,
    left: Option<u64>,
    right: Option<u64>,
    red: bool,
    keys: soroban_sdk::Vec<u64>,
    key_map: Map<u64, u32>,
    count: u64,
}

#[contracttype]

pub struct Tree {
    root: Option<u64>,
    nodes: Map<u64, Node>,
}

impl Tree {
    pub fn new(env: &Env) -> Self {
        Tree {
            root: None,
            nodes: Map::new(env),
        }
    }
}

impl Tree {
    fn first(&self) -> Option<u64> {
        let mut value = self.root;

        while let Some(v) = value {
            let left = self.nodes.get(v)?.left;
            if left.is_none() {
                break;
            }
            value = left;
        }
        value
    }

    fn last(&self) -> Option<u64> {
        let mut value = self.root;
        while let Some(v) = value {
            let right = self.nodes.get(v)?.right;
            if right.is_none() {
                break;
            }
            value = right;
        }
        value
    }

    fn next(&self, value: u64) -> Option<u64> {
        let node = self.nodes.get(value)?;
        if let Some(right) = node.right {
            return self.tree_minimum(right);
        }

        let mut cursor = node.parent;
        let mut current_value = Some(value);

        while let Some(cursor_value) = cursor {
            if let Some(v) = current_value {
                if Some(v) != self.nodes.get(cursor_value)?.right {
                    break;
                }
                current_value = Some(cursor_value);
                cursor = self.nodes.get(cursor_value)?.parent;
            }
        }
        cursor
    }

    fn prev(&self, value: u64) -> Option<u64> {
        let node = self.nodes.get(value)?;
        if let Some(left) = node.left {
            return self.tree_maximum(left);
        }

        let mut cursor = node.parent;
        let mut current_value = Some(value);

        while let Some(cursor_value) = cursor {
            if let Some(v) = current_value {
                if Some(v) != self.nodes.get(cursor_value)?.left {
                    break;
                }
                current_value = Some(cursor_value);
                cursor = self.nodes.get(cursor_value)?.parent;
            }
        }
        cursor
    }

    fn exists(&self, value: u64) -> bool {
        value != 0 && (value == self.root.unwrap_or(0) || self.nodes.contains_key(value))
    }

    fn key_exists(&self, key: u64, value: u64) -> bool {
        if !self.exists(value) {
            return false;
        }

        let node = self.nodes.get(value).unwrap();
        if let Some(index) = node.key_map.get(key) {
            return node.keys.get(index) == Some(key);
        }
        false
    }

    fn get_node(
        &self,
        value: u64,
    ) -> Option<(Option<u64>, Option<u64>, Option<u64>, bool, u32, u64)> {
        if !self.exists(value) {
            return None;
        }

        let node = self.nodes.get(value)?;
        Some((
            node.parent,
            node.left,
            node.right,
            node.red,
            node.keys.len(),
            (node.keys.len() as u64) + node.count,
        ))
    }

    fn tree_minimum(&self, mut value: u64) -> Option<u64> {
        while let Some(left) = self.nodes.get(value)?.left {
            value = left;
        }
        Some(value)
    }

    fn tree_maximum(&self, mut value: u64) -> Option<u64> {
        while let Some(right) = self.nodes.get(value)?.right {
            value = right;
        }
        Some(value)
    }

    fn rotate_left(&mut self, value: u64) {
        if let Some(mut node) = self.nodes.get(value) {
            if let Some(right) = node.right {
                let mut right_node = self.nodes.get(right).unwrap();
                node.right = right_node.left;

                if let Some(left_of_right) = right_node.left {
                    let mut left_node = self.nodes.get_mut(left_of_right).unwrap();
                    left_node.parent = Some(value);
                    // self.nodes.set(left_of_right, left_node); // Commit left_node
                }

                right_node.parent = node.parent;

                if node.parent.is_none() {
                    self.root = Some(right);
                } else if Some(value) == self.nodes.get(node.parent.unwrap()).unwrap().left {
                    self.nodes.get_mut(node.parent.unwrap()).unwrap().left = Some(right);
                } else {
                    self.nodes.get_mut(node.parent.unwrap()).unwrap().right = Some(right);
                }

                right_node.left = Some(value);
                node.parent = Some(right);

                self.nodes.set(right, right_node); // Commit right node
                self.nodes.set(value, node); // Commit node
            }
        }
    }

    fn rotate_right(&mut self, value: u64) {
        if let Some(mut node) = self.nodes.get(value) {
            if let Some(left) = node.left {
                let mut left_node = self.nodes.get(left).unwrap();
                node.left = left_node.right;

                if let Some(right_of_left) = left_node.right {
                    let mut right_node = self.nodes.get_mut(right_of_left).unwrap();
                    right_node.parent = Some(value);
                    // self.nodes.set(right_of_left, right_node); // Commit left_node
                }

                left_node.parent = node.parent;

                if node.parent.is_none() {
                    self.root = Some(left);
                } else if Some(value) == self.nodes.get(node.parent.unwrap()).unwrap().right {
                    self.nodes.get_mut(node.parent.unwrap()).unwrap().right = Some(left);
                } else {
                    self.nodes.get_mut(node.parent.unwrap()).unwrap().left = Some(left);
                }

                left_node.right = Some(value);
                node.parent = Some(left);

                self.nodes.set(left, left_node); // Commit right node
                self.nodes.set(value, node); // Commit node
            }
        }
    }

    fn insert(&mut self, value: u64, key: u64, env: &Env) {
        if value == 0 {
            panic_with_error!(env, Error::OrderStatisticTreeInsert);
        }

        let mut current = self.root;
        let mut parent: Option<u64> = None;
        let mut left_side = false;

        // Finding the right position for the new value
        while let Some(current_value) = current {
            let node = self.nodes.get(current_value).unwrap();
            parent = Some(current_value);

            if value < current_value {
                left_side = true;
                current = node.left;
            } else if value > current_value {
                left_side = false;
                current = node.right;
            } else {
                // Key already exists in the tree
                if !self.key_exists(key, value) {
                    let mut node = self.nodes.get(current_value).unwrap();
                    node.keys.push_back(key);
                    node.key_map.set(key, node.keys.len() - 1);
                    node.count += 1;
                    self.nodes.set(current_value, node);
                }
                return;
            }
        }

        // Inserting the new node
        let new_node = Node {
            parent,
            left: None,
            right: None,
            red: true, // All newly inserted nodes are red by default in red-black tree
            keys: soroban_sdk::vec![env, key],
            key_map: soroban_sdk::map![env, (key, 0)],
            count: 1,
        };

        self.nodes.set(value, new_node);

        // Linking the parent node
        if let Some(parent_value) = parent {
            let mut parent_node = self.nodes.get(parent_value).unwrap();
            if left_side {
                parent_node.left = Some(value);
            } else {
                parent_node.right = Some(value);
            }
            self.nodes.set(parent_value, parent_node);
        } else {
            // If there is no parent, the inserted node is the root
            self.root = Some(value);
        }

        // Fixing up the tree to maintain the red-black properties
        self.insert_fixup(value);
    }

    fn insert_fixup(&mut self, mut value: u64) {
        while let Some(parent_value) = self.nodes.get(value).unwrap().parent {
            if let Some(grandparent_value) = self.nodes.get(parent_value).unwrap().parent {
                // fix
                let is_parent_left =
                    self.nodes.get(grandparent_value).unwrap().left == Some(parent_value);
                let uncle = if is_parent_left {
                    self.nodes.get(grandparent_value).unwrap().right
                } else {
                    self.nodes.get(grandparent_value).unwrap().left
                };

                if uncle.is_some() && self.nodes.get(uncle.unwrap()).unwrap().red {
                    // Case 1: Uncle is red
                    // TODO fix save state
                    self.nodes.get(parent_value).unwrap().red = false;
                    self.nodes.get(uncle.unwrap()).unwrap().red = false;
                    self.nodes.get(grandparent_value).unwrap().red = true;
                    value = grandparent_value;
                } else {
                    // Case 2: Uncle is black and the node is on the opposite side of the parent
                    if is_parent_left && self.nodes.get(parent_value).unwrap().right == Some(value)
                    {
                        value = parent_value;
                        self.rotate_left(value);
                    } else if !is_parent_left
                        && self.nodes.get(parent_value).unwrap().left == Some(value)
                    {
                        value = parent_value;
                        self.rotate_right(value);
                    }

                    // Case 3: Uncle is black and the node is on the same side as the parent
                    // TODO fix save state
                    self.nodes.get(parent_value).unwrap().red = false;
                    self.nodes.get(grandparent_value).unwrap().red = true;
                    if is_parent_left {
                        self.rotate_right(grandparent_value);
                    } else {
                        self.rotate_left(grandparent_value);
                    }
                }
            } else {
                break;
            }
        }

        // The root is always black
        // TODO fix handle the storage
        // self.nodes.get(&self.root.unwrap()).unwrap().red = false;
    }

    fn remove(&mut self, value: u64, key: u64) {
        if !self.exists(value) || !self.key_exists(key, value) {
            return;
        }

        let mut node = self.nodes.get(value).unwrap();

        // Remove key from the node
        if let Some(index) = node.key_map.get(key) {
            node.keys.remove(index);
            node.key_map.remove(key);
            node.count -= 1;

            // If no more keys, remove the node
            if node.keys.is_empty() {
                self.remove_node(value);
            }

            self.nodes.set(value, node);
        }
    }

    fn remove_node(&mut self, value: u64) {
        let node = self.nodes.get(value).unwrap();
        // let mut y = value;
        let mut y_original_color = node.red;
        let x: Option<u64>;

        if node.left.is_none() {
            x = node.right;
            self.transplant(value, node.right);
        } else if node.right.is_none() {
            x = node.left;
            self.transplant(value, node.left);
        } else {
            let min_right = self.tree_minimum(node.right.unwrap()).unwrap();
            // y = min_right;
            y_original_color = self.nodes.get(min_right).unwrap().red;
            x = self.nodes.get(min_right).unwrap().right;

            if self.nodes.get(min_right).unwrap().parent == Some(value) {
                if let Some(x_val) = x {
                    self.nodes.get_mut(x_val).unwrap().parent = Some(min_right);
                }
            } else {
                self.transplant(min_right, x);
                self.nodes.get_mut(min_right).unwrap().right = node.right;
                self.nodes.get_mut(min_right).unwrap().parent = Some(min_right);
            }

            self.transplant(value, Some(min_right));
            self.nodes.get_mut(min_right).unwrap().left = node.left;
            self.nodes.get_mut(min_right).unwrap().parent = Some(min_right);
            self.nodes.get_mut(min_right).unwrap().red = node.red;
        }

        if !y_original_color {
            self.remove_fixup(x);
        }
    }

    fn transplant(&mut self, u: u64, v: Option<u64>) {
        if self.nodes.get(u).unwrap().parent == None {
            self.root = v;
        } else if self
            .nodes
            .get(self.nodes.get(u).unwrap().parent.unwrap())
            .and_then(|node| node.left)
            == Some(u)
        {
            self.nodes
                .get_mut(self.nodes.get(u).unwrap().parent.unwrap())
                .unwrap()
                .left = v;
        } else {
            self.nodes
                .get_mut(self.nodes.get(u).unwrap().parent.unwrap())
                .unwrap()
                .right = v;
        }

        if let Some(v_val) = v {
            self.nodes.get_mut(v_val).unwrap().parent = self.nodes.get(u).unwrap().parent;
        }
    }

    fn remove_fixup(&mut self, mut x: Option<u64>) {
        while let Some(x_val) = x {
            if self.root == Some(x_val) || self.nodes.get(x_val).unwrap().red {
                break;
            }

            let mut w: Option<u64>;
            let parent_val = self.nodes.get(x_val).unwrap().parent.unwrap();

            if self.nodes.get(parent_val).unwrap().left == Some(x_val) {
                w = self.nodes.get(parent_val).unwrap().right;
                if let Some(w_val) = w {
                    if self.nodes.get(w_val).unwrap().red {
                        self.nodes.get_mut(w_val).unwrap().red = false;
                        self.nodes.get_mut(parent_val).unwrap().red = true;
                        self.rotate_left(parent_val);
                        w = self.nodes.get(parent_val).unwrap().right;
                    }

                    if !self.nodes.get_left_of(w_val).unwrap().red
                        && !self.nodes.get_right_of(w_val).unwrap().red
                    {
                        self.nodes.get_mut(w_val).unwrap().red = true;
                        x = Some(parent_val);
                    } else {
                        if !self.nodes.get_right_of(w_val).unwrap().red {
                            self.nodes.get_left_of_mut(w_val).unwrap().red = false;
                            self.nodes.get_mut(w_val).unwrap().red = true;
                            self.rotate_right(w_val);
                            w = self.nodes.get(parent_val).unwrap().right;
                        }

                        if let Some(w_val) = w {
                            self.nodes.get_mut(w_val).unwrap().red =
                                self.nodes.get(parent_val).unwrap().red;
                            self.nodes.get_mut(parent_val).unwrap().red = false;
                            self.nodes.get_right_of_mut(w_val).unwrap().red = false;
                            self.rotate_left(parent_val);
                        }
                        x = self.root;
                    }
                }
            } else {
                w = self.nodes.get(parent_val).unwrap().left;
                if let Some(w_val) = w {
                    if self.nodes.get(w_val).unwrap().red {
                        self.nodes.get_mut(w_val).unwrap().red = false;
                        self.nodes.get_mut(parent_val).unwrap().red = true;
                        self.rotate_right(parent_val);
                        w = self.nodes.get(parent_val).unwrap().left;
                    }

                    if !self.nodes.get_right_of(w_val).unwrap().red
                        && !self.nodes.get_left_of(w_val).unwrap().red
                    {
                        self.nodes.get_mut(w_val).unwrap().red = true;
                        x = Some(parent_val);
                    } else {
                        if !self.nodes.get_left_of(w_val).unwrap().red {
                            self.nodes.get_right_of_mut(w_val).unwrap().red = false;
                            self.nodes.get_mut(w_val).unwrap().red = true;
                            self.rotate_left(w_val);
                            w = self.nodes.get(parent_val).unwrap().left;
                        }

                        if let Some(w_val) = w {
                            self.nodes.get_mut(w_val).unwrap().red =
                                self.nodes.get(parent_val).unwrap().red;
                            self.nodes.get_mut(parent_val).unwrap().red = false;
                            self.nodes.get_left_of_mut(w_val).unwrap().red = false;
                            self.rotate_right(parent_val);
                        }
                        x = self.root;
                    }
                }
            }
        }

        if let Some(x_val) = x {
            self.nodes.get_mut(x_val).unwrap().red = false;
        }
    }
}
