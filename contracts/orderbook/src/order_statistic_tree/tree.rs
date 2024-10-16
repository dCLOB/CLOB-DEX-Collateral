use core::u64;

use super::node::{InMemoryNode, NodeViewHolder, StorageAccessor};
use crate::error::Error;
use crate::order_statistic_tree::node::ColorInterface;
use crate::order_statistic_tree::node::NodeViewInterface;

pub struct OrderStatisticTree<T: StorageAccessor> {
    pub(crate) root: NodeViewHolder<T>,
    pub storage_accessor: T,
}

impl<T: StorageAccessor> OrderStatisticTree<T> {
    pub fn new(storage_accessor: T) -> Self {
        OrderStatisticTree {
            root: NodeViewHolder::new(T::NodeViewT::empty_node(), storage_accessor),
            storage_accessor,
        }
    }

    pub fn from_root(root: T::NodeViewT, storage_accessor: T) -> Self {
        OrderStatisticTree {
            root: storage_accessor.to_node_holder(root),
            storage_accessor: storage_accessor,
        }
    }
}

macro_rules! color {
    (Black) => {
        T::NodeViewT::black()
    };
    (Red) => {
        T::NodeViewT::red()
    };
}

impl<T: StorageAccessor + Copy> OrderStatisticTree<T> {
    pub fn first(&self) -> Result<T::NodeViewT, Error> {
        let mut value = self.root;

        while !value.is_empty() {
            let left = value.load()?.left();
            if left.is_empty() {
                break;
            }
            value = left;
        }

        Ok(value.get_view())
    }

    pub fn last(&self) -> Result<T::NodeViewT, Error> {
        let mut value = self.root;
        while !value.is_empty() {
            let right = value.load()?.right();
            if right.is_empty() {
                break;
            }
            value = right;
        }

        Ok(value.get_view())
    }

    pub fn next(&self, node: NodeViewHolder<T>) -> Result<NodeViewHolder<T>, Error> {
        let node = node.load()?;
        if !node.right().is_empty() {
            return self.tree_minimum(node.right());
        }

        let mut cursor = node.parent();
        let mut current_value = node.node_view();

        while !cursor.is_empty() {
            if current_value != cursor.load()?.right() {
                break;
            }

            current_value = cursor;
            cursor = cursor.load()?.parent();
        }

        Ok(cursor)
    }

    pub fn prev(&self, node: NodeViewHolder<T>) -> Result<NodeViewHolder<T>, Error> {
        let node = node.load()?;
        if !node.left().is_empty() {
            return self.tree_minimum(node.left());
        }

        let mut cursor = node.parent();
        let mut current_value = node.node_view();

        while !cursor.is_empty() {
            if current_value != cursor.load()?.right() {
                break;
            }

            current_value = cursor;
            cursor = cursor.load()?.parent();
        }

        Ok(cursor)
    }

    pub fn exists(&self, value: u64) -> bool {
        let node_view = T::NodeViewT::new(value);
        let contains = self.storage_accessor.node_exists(node_view);
        let res = value != 0 && (node_view == *self.root || contains);
        res
    }

    pub fn key_exists(&self, key: u64, value: u64) -> bool {
        if !self.exists(value) {
            return false;
        }

        let node_view = self
            .storage_accessor
            .to_node_holder(T::NodeViewT::new(value));

        node_view.load().unwrap().key_exists(key)
    }

    fn tree_minimum(&self, node: NodeViewHolder<T>) -> Result<NodeViewHolder<T>, Error> {
        let mut cursor = node;
        while !cursor.is_empty() {
            cursor = cursor.load()?.left();
        }
        Ok(cursor)
    }

    #[allow(dead_code)]
    fn tree_maximum(&self, node: NodeViewHolder<T>) -> Result<NodeViewHolder<T>, Error> {
        let mut cursor = node;
        while !cursor.is_empty() {
            cursor = cursor.load()?.left();
        }

        Ok(cursor)
    }

    fn rotate_left(&mut self, node_view: NodeViewHolder<T>) -> Result<(), Error> {
        let mut node = node_view.load()?;
        let old_parent = node.parent();

        let mut right_child = node.right().load()?;

        *node.right_mut() = *right_child.left();
        if !right_child.left().is_empty() {
            *right_child.left().load()?.parent_mut() = *node_view;
        }

        *right_child.left_mut() = *node_view;
        *node.parent_mut() = *right_child;

        let node_view = node.sync()?;
        let right_child = right_child.sync()?;

        self.replace_parent_child_relations(old_parent, node_view, right_child)?;

        Ok(())
    }

    fn rotate_right(&mut self, node_view: NodeViewHolder<T>) -> Result<(), Error> {
        let mut node = node_view.load()?;
        let old_parent = node.parent();

        let mut left_child = node.left().load()?;

        *node.left_mut() = *left_child.right();
        if !left_child.right().is_empty() {
            *left_child.right().load()?.parent_mut() = *node_view;
        }

        *left_child.right_mut() = *node_view;
        *node.parent_mut() = *left_child;

        let node_view = node.sync()?;
        let left_child = left_child.sync()?;

        self.replace_parent_child_relations(old_parent, node_view, left_child)?;

        Ok(())
    }

    fn replace_parent_child_relations(
        &mut self,
        parent: NodeViewHolder<T>,
        old_child: NodeViewHolder<T>,
        new_child: NodeViewHolder<T>,
    ) -> Result<(), Error> {
        if parent.is_empty() {
            self.root = new_child;
        } else {
            let mut parent = parent.load()?;

            if parent.left() == old_child {
                *parent.left_mut() = *new_child;
            } else if parent.right() == old_child {
                *parent.right_mut() = *new_child;
            } else {
                return Err(Error::NotAChildOfItsParent);
            }
        }

        if !new_child.is_empty() {
            let mut new_child = new_child.load()?;
            *new_child.parent_mut() = *parent;
        }

        Ok(())
    }

    fn replace_parent_relation(
        &mut self,
        parent: NodeViewHolder<T>,
        old_child: NodeViewHolder<T>,
        new_child: NodeViewHolder<T>,
    ) -> Result<(), Error> {
        if parent.is_empty() {
            self.root = new_child;
        } else {
            let mut parent = parent.load()?;

            if parent.left() == old_child {
                *parent.left_mut() = *new_child;
            } else if parent.right() == old_child {
                *parent.right_mut() = *new_child;
            } else {
                return Err(Error::NotAChildOfItsParent);
            }
        }

        Ok(())
    }

    fn replace_child_relation(
        &mut self,
        child: NodeViewHolder<T>,
        old_parent: NodeViewHolder<T>,
        new_parent: NodeViewHolder<T>,
    ) -> Result<(), Error> {
        if !child.is_empty() {
            let mut child_node = child.load()?;

            if child_node.parent() == old_parent {
                *child_node.parent_mut() = *new_parent;
            } else {
                return Err(Error::NotAParentOfChild);
            }
        }

        Ok(())
    }

    pub fn insert(&mut self, value: u64, key: u64) -> Result<(), Error> {
        if value == 0 {
            return Err(Error::ZeroValueInsert);
        }

        let node_value = T::NodeViewT::new(value);

        let mut current = self.root;
        let mut parent = self.root;
        let mut left_side = false;

        // Finding the right position for the new value
        while !current.is_empty() {
            let mut cur_node = current.load()?;
            parent = cur_node.node_view();

            if node_value < *cur_node {
                left_side = true;
                current = cur_node.left();
            } else if node_value > *cur_node {
                left_side = false;
                current = cur_node.right();
            } else {
                // Key already exists in the tree
                if !cur_node.key_exists(key) {
                    cur_node.insert_key(key);
                }
                return Ok(());
            }
        }

        // Inserting the new node
        self.storage_accessor.new_node(*parent, node_value, key)?;

        // Linking the parent node
        if !parent.is_empty() {
            if left_side {
                let mut parent = parent.load()?;

                *parent.left_mut() = node_value;
            } else {
                *parent.load()?.right_mut() = node_value;
            }
        } else {
            // If there is no parent, the inserted node is the root
            self.root =
                self.storage_accessor
                    .new_node(T::NodeViewT::empty_node(), node_value, key)?;
        }

        // Fixing up the tree to maintain the color-black properties
        self.insert_fixup(self.storage_accessor.to_node_holder(node_value))?;

        Ok(())
    }

    fn insert_fixup(&mut self, current_node: NodeViewHolder<T>) -> Result<(), Error> {
        let mut current_node = current_node.load()?;
        let parent = current_node.parent();

        // Case 1: Parent is null, we've reached the root, the end of the recursion
        if parent.is_empty() {
            // Uncomment the following line if you want to enforce black roots (rule 2):
            *current_node.color_mut() = T::NodeViewT::black();
            return Ok(());
        }

        let mut parent = parent.load()?;

        // Parent is black --> nothing to do
        if parent.color() == color!(Black) {
            return Ok(());
        }

        // From here on, parent is red
        let grandparent = parent.parent();

        // Case 2:
        // Not having a grandparent means that parent is the root. If we enforce black roots
        // (rule 2), grandparent will never be null, and the following if-then block can be
        // removed.
        if grandparent.is_empty() {
            // As this method is only called on red nodes (either on newly inserted ones - or -
            // recursively on red grandparents), all we have to do is to recolor the root black.
            *parent.color_mut() = color!(Black);
            return Ok(());
        }

        let grand_parent = grandparent.load()?;
        // Get the uncle (may be null/nil, in which case its color is BLACK)
        let uncle = self.get_uncle(&parent, &grand_parent)?;

        let uncle_is_empty = uncle.is_empty();

        // Case 3: Uncle is red -> recolor parent, grandparent and uncle
        if !uncle_is_empty && uncle.clone().load()?.color() == color!(Red) {
            {
                // we need to do this in scope in order to immediately apply the changes
                *uncle.load()?.color_mut() = color!(Black);
                *parent.node_view().load()?.color_mut() = color!(Black);
                *grand_parent.node_view().load()?.color_mut() = color!(Red);
            }

            // Call recursively for grandparent, which is now red.
            // It might be root or have a red parent, in which case we need to fix more...
            self.insert_fixup(grand_parent.node_view())?;
        }
        // Note on performance:
        // It would be faster to do the uncle color check within the following code. This way
        // we would avoid checking the grandparent-parent direction twice (once in getUncle()
        // and once in the following else-if). But for better understanding of the code,
        // I left the uncle color check as a separate step.

        // Parent is left child of grandparent
        else if *parent == *grand_parent.left() {
            // Case 4a: Uncle is black and node is left->right "inner child" of its grandparent
            if *current_node == *parent.right() {
                self.rotate_left(parent.node_view())?;

                // Let "parent" point to the new root node of the rotated sub-tree.
                // It will be recolored in the next step, which we're going to fall-through to.
                parent = current_node;
            }

            // Case 5a: Uncle is black and node is left->left "outer child" of its grandparent
            self.rotate_right(grand_parent.node_view())?;

            // Recolor original parent and grandparent
            *parent.node_view().load()?.color_mut() = color!(Black);
            *grand_parent.node_view().load()?.color_mut() = color!(Red);
        }
        // Parent is right child of grandparent
        else {
            // Case 4b: Uncle is black and node is right->left "inner child" of its grandparent
            if *current_node == *parent.left() {
                self.rotate_right(parent.node_view())?;

                // Let "parent" point to the new root node of the rotated sub-tree.
                // It will be recolored in the next step, which we're going to fall-through to.
                parent = current_node;
            }

            // Case 5b: Uncle is black and node is right->right "outer child" of its grandparent
            self.rotate_left(grand_parent.node_view())?;

            // Recolor original parent and grandparent
            *parent.node_view().load()?.color_mut() = color!(Black);
            *grand_parent.node_view().load()?.color_mut() = color!(Red);
        }

        Ok(())
    }

    fn get_uncle(
        &self,
        parent: &InMemoryNode<T>,
        grandparent: &InMemoryNode<T>,
    ) -> Result<NodeViewHolder<T>, Error> {
        let parent_view = parent.node_view();
        if grandparent.left() == parent_view {
            Ok(grandparent.right())
        } else if grandparent.right() == parent_view {
            Ok(grandparent.left())
        } else {
            Err(Error::NotAChildOfItsParent)
        }
    }

    pub fn remove(&mut self, value: u64, key: u64) -> Result<(), Error> {
        if !self.exists(value) || !self.key_exists(key, value) {
            return Ok(());
        }

        let mut node = self
            .storage_accessor
            .to_node_holder(T::NodeViewT::new(value))
            .load()?;

        // Remove key from the node
        node.remove_key(key);

        // If no more keys, remove the node
        if node.keys_empty() {
            self.remove_node(node.node_view())?;
        }

        Ok(())
    }

    pub fn remove_node(&mut self, value: NodeViewHolder<T>) -> Result<(), Error> {
        if self.root.is_empty() {
            return Ok(());
        }

        let mut node_view = self.root;

        // Find the node to be deleted
        while !node_view.is_empty() && node_view != value {
            // Traverse the tree to the left or right depending on the key
            if *value < *node_view {
                node_view = node_view.load()?.left();
            } else {
                node_view = node_view.load()?.right();
            }
        }

        let mut node = node_view.load()?;
        // At this point, "node" is the node to be deleted

        // In this variable, we'll store the node at which we're going to start to fix the R-B
        // properties after deleting a node.
        let moved_up_node_id;
        let deleted_node_color;

        // Node has zero or one child
        if node.left().is_empty() || node.right().is_empty() {
            moved_up_node_id = self.delete_node_with_zero_or_one_child(node.node_view())?;
            deleted_node_color = node.color();
            self.storage_accessor.remove_node(*node)?;
        }
        // Node has two children
        else {
            // Find maximum node of left subtree ("inorder successor" of current node)
            let in_order_successor = self.find_maximum(node.left())?;

            if in_order_successor == node.left() {
                let mut in_order_successor = in_order_successor.load()?;
                deleted_node_color = in_order_successor.color();

                *in_order_successor.color_mut() = node.color();
                *in_order_successor.right_mut() = *node.right();
                *in_order_successor.left_mut() = *node; // This is a workaround but it breaks the left is less structure
                *in_order_successor.parent_mut() = *node.parent();

                self.replace_parent_relation(
                    node.parent(),
                    node.node_view(),
                    in_order_successor.node_view(),
                )?;

                self.replace_child_relation(
                    node.right(),
                    node.node_view(),
                    in_order_successor.node_view(),
                )?;

                *node.left_mut() = T::NodeViewT::empty_node();
                *node.right_mut() = T::NodeViewT::empty_node();
                *node.parent_mut() = *in_order_successor;
                *node.color_mut() = deleted_node_color;

                in_order_successor.sync()?;
                let node = node.sync()?;

                moved_up_node_id = self.delete_node_with_zero_or_one_child(node)?;
                self.storage_accessor.remove_node(*node)?;
            } else {
                let mut in_order_successor = in_order_successor.load()?;
                let in_order_successor_parent = in_order_successor.parent();
                deleted_node_color = in_order_successor.color();

                self.replace_parent_relation(
                    node.parent(),
                    node.node_view(),
                    in_order_successor.node_view(),
                )?;

                self.replace_parent_relation(
                    in_order_successor.parent(),
                    in_order_successor.node_view(),
                    node.node_view(),
                )?;

                self.replace_child_relation(
                    node.left(),
                    node.node_view(),
                    in_order_successor.node_view(),
                )?;

                self.replace_child_relation(
                    node.right(),
                    node.node_view(),
                    in_order_successor.node_view(),
                )?;

                *in_order_successor.left_mut() = *node.left();
                *in_order_successor.right_mut() = *node.right();
                *in_order_successor.parent_mut() = *in_order_successor_parent;
                *in_order_successor.color_mut() = node.color();

                *node.left_mut() = T::NodeViewT::empty_node();
                *node.right_mut() = T::NodeViewT::empty_node();
                *node.parent_mut() = *in_order_successor_parent;
                *node.color_mut() = deleted_node_color;

                in_order_successor.sync()?;
                let node = node.sync()?;

                // Delete inorder successor just as we would delete a node with 0 or 1 child
                moved_up_node_id = self.delete_node_with_zero_or_one_child(node)?;
                self.storage_accessor.remove_node(*node)?;
            }
        }

        if deleted_node_color == color!(Black) {
            self.fix_red_black_properties_after_delete(moved_up_node_id)?;

            // Remove the temporary NIL node
            if *moved_up_node_id == T::NodeViewT::nil_node() {
                if moved_up_node_id == self.root {
                    self.storage_accessor.remove_node(*node_view)?;
                    self.root = self
                        .storage_accessor
                        .to_node_holder(T::NodeViewT::empty_node());
                } else {
                    let move_up_node = moved_up_node_id.load()?;
                    let mut parent = move_up_node.parent().load()?;

                    if parent.left() == moved_up_node_id {
                        *parent.left_mut() = T::NodeViewT::empty_node();
                    } else if parent.right() == moved_up_node_id {
                        *parent.right_mut() = T::NodeViewT::empty_node();
                    } else {
                        return Err(Error::NotAParentOfChild);
                    }
                }
            }
        }

        Ok(())
    }

    fn delete_node_with_zero_or_one_child(
        &mut self,
        node_view: NodeViewHolder<T>,
    ) -> Result<NodeViewHolder<T>, Error> {
        let node = node_view.load()?;

        // Node has ONLY a left child --> replace by its left child
        if !node.left().is_empty() {
            self.replace_parent_child_relations(node.parent(), node.node_view(), node.left())?;
            return Ok(node.left()); // moved-up node
        }
        // Node has ONLY a right child --> replace by its right child
        else if !node.right().is_empty() {
            self.replace_parent_child_relations(node.parent(), node.node_view(), node.right())?;
            return Ok(node.right()); // moved-up node
        }
        // Node has no children -->
        // * node is red --> just remove it
        // * node is black --> replace it by a temporary NIL node (needed to fix the R-B rules)
        else {
            if node.color() == color!(Black) {
                let nil_node = self.storage_accessor.create_nil_node(*node.parent())?;
                self.replace_parent_child_relations(node.parent(), node.node_view(), nil_node)?; // nil node added to the storage
                Ok(nil_node)
            } else {
                let mut parent = node.parent().load()?;

                if *node == *parent.left() {
                    *parent.left_mut() = T::NodeViewT::empty_node();
                } else if *node == *parent.right() {
                    *parent.right_mut() = T::NodeViewT::empty_node();
                } else {
                    return Err(Error::NotAChildOfItsParent);
                }
                Ok(node.node_view())
            }
        }
    }

    fn find_maximum(&self, node: NodeViewHolder<T>) -> Result<NodeViewHolder<T>, Error> {
        let mut node = node;
        while let Some(res) = node.load().ok().and_then(|el| {
            if !el.right().is_empty() {
                Some(el)
            } else {
                None
            }
        }) {
            node = res.right();
        }

        Ok(node)
    }

    fn fix_red_black_properties_after_delete(
        &mut self,
        node: NodeViewHolder<T>,
    ) -> Result<(), Error> {
        // Case 1: Examined node is root, end of recursion
        if !node.is_empty() && node == self.root {
            // Ensure the root is black (rule 2)
            *node.load()?.color_mut() = color!(Black);
            return Ok(());
        }

        // Get sibling of the node
        let mut sibling = self.get_sibling(node)?.load()?;

        // Case 2: Red sibling
        if sibling.color() == color!(Red) {
            self.handle_red_sibling(node, sibling.node_view())?;
            sibling = self.get_sibling(node)?.load()?; // Refresh sibling for fall-through to cases 3-6
        }

        // Cases 3+4: Black sibling with two black children
        if self.is_node_black_or_none(sibling.left()) && self.is_node_black_or_none(sibling.right())
        {
            *sibling.color_mut() = color!(Red);
            sibling.sync()?;

            let node = node.load()?;
            let mut parent = node.parent().load()?;

            // Case 3: Black sibling with two black children + red parent
            if parent.color() == color!(Red) {
                *parent.color_mut() = color!(Black);
                parent.sync()?;
            }
            // Case 4: Black sibling with two black children + black parent
            else {
                self.fix_red_black_properties_after_delete(parent.node_view())?;
            }
        }
        // Cases 5+6: Black sibling with at least one red child
        else {
            self.handle_black_sibling_with_at_least_one_red_child(node, sibling.node_view())?;
        }

        Ok(())
    }

    fn handle_red_sibling(
        &mut self,
        node_id: NodeViewHolder<T>,
        sibling_id: NodeViewHolder<T>,
    ) -> Result<(), Error> {
        // Load the sibling node
        let mut sibling_node = sibling_id.load()?;

        // Get the parent of the current node
        let mut parent = node_id.load()?.parent().load()?;
        let parent_left = parent.left();

        // Recolor the sibling to black and the parent to red
        *sibling_node.color_mut() = color!(Black);
        *parent.color_mut() = color!(Red);

        // Sync the changes to the nodes
        sibling_node.sync()?;
        let parent_node = parent.sync()?;

        // Perform rotations based on whether the node is the left or right child of the parent
        if node_id == parent_left {
            self.rotate_left(parent_node)?;
        } else {
            self.rotate_right(parent_node)?;
        }

        Ok(())
    }

    fn handle_black_sibling_with_at_least_one_red_child(
        &mut self,
        node: NodeViewHolder<T>,
        sibling_view: NodeViewHolder<T>,
    ) -> Result<(), Error> {
        let mut sibling_view = sibling_view;
        let mut sibling = sibling_view.load()?;
        // let parent_id = self.nodes.get(node_id).unwrap().parent.unwrap();
        let mut parent = node.load()?.parent().load()?;
        // let mut sibling = self.nodes.get(sibling_id).unwrap();
        let node_is_left_child = node == parent.left();

        // Case 5: Black sibling with at least one red child + "outer nephew" is black
        // --> Recolor sibling and its child, and rotate around sibling
        if node_is_left_child && self.is_node_black_or_none(sibling.right()) {
            *sibling.left().load()?.color_mut() = color!(Black);
            *sibling.color_mut() = color!(Red);

            let sibling = sibling.sync()?;
            self.rotate_right(sibling)?;

            parent = parent.node_view().load()?;
            sibling_view = parent.right();
        } else if !node_is_left_child && self.is_node_black_or_none(sibling.left()) {
            *sibling.right().load()?.color_mut() = color!(Black);
            *sibling.color_mut() = color!(Red);

            let sibling = sibling.sync()?;
            self.rotate_left(sibling)?;

            parent = parent.node_view().load()?;
            sibling_view = parent.left();
        }

        // Fall-through to case 6...

        // Case 6: Black sibling with at least one red child + "outer nephew" is red
        // --> Recolor sibling + parent + sibling's child, and rotate around parent
        // let mut parent = self.nodes.get(parent_id).unwrap();
        let mut sibling = sibling_view.load()?;
        *sibling.color_mut() = parent.color();
        *parent.color_mut() = color!(Black);

        let parent = parent.sync()?;
        sibling.sync()?;

        if node_is_left_child {
            let mut sibling_right = sibling_view.load()?.right().load()?;
            *sibling_right.color_mut() = color!(Black);

            sibling_right.sync()?;
            self.rotate_left(parent)?;
        } else {
            let mut sibling_left = sibling_view.load()?.left().load()?;
            *sibling_left.color_mut() = color!(Black);

            sibling_left.sync()?;
            self.rotate_right(parent)?;
        }

        Ok(())
    }

    fn get_sibling(&self, node_view: NodeViewHolder<T>) -> Result<NodeViewHolder<T>, Error> {
        let node = node_view.load()?;
        let parent = node.parent().load()?;

        if node_view == parent.left() {
            return Ok(parent.right());
        } else if node_view == parent.right() {
            return Ok(parent.left());
        }

        Err(Error::NotAParentOfChild)
    }

    fn is_node_black_or_none(&self, node_id: NodeViewHolder<T>) -> bool {
        node_id.is_empty()
            || node_id
                .load()
                .map(|n| n.color() == color!(Black))
                .unwrap_or(false)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod tree_printer {
    use crate::order_statistic_tree::node::{NodeViewHolder, StorageAccessor};

    #[allow(dead_code)]
    fn print_2d_util<T: StorageAccessor>(node: NodeViewHolder<T>, space: usize) {
        const COUNT: usize = 5; // Define your space count
                                // Base case
        if !node.is_empty() {
            let node = node.load().unwrap();
            // Increase distance between levels
            let space = space + COUNT;

            // Process right child first
            print_2d_util(node.right(), space);

            // Print current node after space count
            std::println!();
            for _ in COUNT..space {
                std::print!(" ");
            }
            std::println!(
                "Id {:?},parent{:?},left:{:?},right:{:?},color:{:?}",
                *node.node_view(),
                *node.parent(),
                *node.left(),
                *node.right(),
                node.color()
            );

            // Process left child
            print_2d_util(node.left(), space);
        }
    }

    #[allow(dead_code)]
    // Wrapper over print_2d_util()
    fn print_2d<T: StorageAccessor>(root: NodeViewHolder<T>) {
        // Pass initial space count as 0
        print_2d_util(root, 0);
        println!();
        println!();
        println!();
    }
}
