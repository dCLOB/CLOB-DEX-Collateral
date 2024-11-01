use core::fmt::Debug;
use soroban_sdk::{contracttype, Env};

use crate::{
    error::Error,
    order_statistic_tree::node::{
        ColorInterface, InMemoryNode, Key, NodeId, NodeInterface, NodeViewHolder,
        NodeViewInterface, StorageAccessor,
    },
};

#[derive(Clone)]
#[contracttype]
pub struct NodeKey {
    id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Copy)]
#[contracttype]
pub enum NodeColor {
    Red,
    Black,
}

#[contracttype]
#[derive(Default, Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeView {
    id: Option<NodeId>,
}

impl NodeView {
    pub fn new(id: NodeId) -> Self {
        Self { id: Some(id) }
    }
}

impl NodeViewInterface for NodeView {
    fn is_empty(&self) -> bool {
        self.id.is_none()
    }

    fn new(id: NodeId) -> Self {
        NodeView::new(id)
    }

    fn empty_node() -> Self {
        NodeView { id: None }
    }

    fn nil_node() -> Self {
        NodeView {
            id: Some(NodeId::MAX),
        }
    }

    fn to_raw(&self) -> Option<NodeId> {
        self.id
    }
}

impl TryInto<NodeKey> for NodeView {
    type Error = Error;

    fn try_into(self) -> Result<NodeKey, Self::Error> {
        self.id.map(|id| NodeKey { id }).ok_or(Error::EmptyNodeView)
    }
}

impl ColorInterface for NodeView {
    type ColorType = NodeColor;

    fn black() -> Self::ColorType {
        NodeColor::Black
    }

    fn red() -> Self::ColorType {
        NodeColor::Red
    }
}

impl<'a> StorageAccessor for &'a Env {
    type InnerNodeT = InnerNode;
    type NodeViewT = NodeView;

    fn load(&self, node_view: NodeView) -> Result<InMemoryNode<Self>, Error> {
        let key: NodeKey = node_view.try_into()?;

        self.storage()
            .temporary()
            .get(&key)
            .map(|in_node| InMemoryNode {
                is_modified: false,
                id: node_view,
                inner_node: in_node,
                storage_accessor: *self,
            })
            .ok_or(Error::EmptyNodeView)
    }

    fn upload(&self, node_view: NodeView, in_node: &InnerNode) -> Result<(), Error> {
        let key: NodeKey = node_view.try_into()?;

        self.storage().temporary().set(&key, in_node);

        Ok(())
    }

    fn new_node(
        &self,
        parent: NodeView,
        id: NodeView,
        key: Key,
    ) -> Result<NodeViewHolder<Self>, Error> {
        let node = InnerNode {
            parent: parent,
            left: NodeView::empty_node(),
            right: NodeView::empty_node(),
            color: NodeColor::Red, // All newly inserted nodes are color by default in color-black tree
            keys: soroban_sdk::vec![self, key],
            key_map: soroban_sdk::map![self, (key, 0)],
            count: 1,
        };

        self.upload(id, &node)?;

        Ok(NodeViewHolder::new(id, *self))
    }

    fn remove_node(&self, node_view: NodeView) -> Result<(), Error> {
        let key: NodeKey = node_view.try_into()?;

        self.storage().temporary().remove(&key);

        Ok(())
    }

    fn to_node_holder(&self, node_view: NodeView) -> NodeViewHolder<Self> {
        NodeViewHolder::new(node_view, *self)
    }

    fn node_exists(&self, node_view: NodeView) -> bool {
        self.storage()
            .temporary()
            .get::<Self::NodeViewT, InnerNode>(&node_view.try_into().unwrap())
            .is_some()
    }

    fn create_nil_node(&self, parent: NodeView) -> Result<NodeViewHolder<Self>, Error> {
        let node_view = Self::NodeViewT::nil_node();

        let inner = InnerNode {
            parent,
            left: Self::NodeViewT::empty_node(),
            right: Self::NodeViewT::empty_node(),
            color: NodeColor::Black,
            keys: soroban_sdk::vec![*self],
            key_map: soroban_sdk::map![*self],
            count: 1,
        };

        self.upload(node_view, &inner)?;

        Ok(self.to_node_holder(node_view))
    }
}

#[contracttype]
#[derive(Clone, Debug)]

pub struct InnerNode {
    parent: NodeView,
    left: NodeView,
    right: NodeView,
    color: NodeColor,
    keys: soroban_sdk::Vec<Key>,
    key_map: soroban_sdk::Map<Key, u32>,
    count: u64,
}

impl NodeInterface<NodeView> for InnerNode {
    fn right_mut(&mut self) -> &mut NodeView {
        &mut self.right
    }

    fn left_mut(&mut self) -> &mut NodeView {
        &mut self.left
    }

    fn parent_mut(&mut self) -> &mut NodeView {
        &mut self.parent
    }

    fn key_exists(&self, key: Key) -> bool {
        self.key_map.contains_key(key)
    }

    fn insert_key(&mut self, key: Key) {
        self.keys.push_back(key);
        self.key_map.set(key, self.keys.len() - 1);
        self.count += 1;
    }

    fn left(&self) -> NodeView {
        self.left
    }

    fn right(&self) -> NodeView {
        self.right
    }

    fn parent(&self) -> NodeView {
        self.parent
    }

    fn color_mut(&mut self) -> &mut <NodeView as ColorInterface>::ColorType {
        &mut self.color
    }

    fn color(&self) -> <NodeView as ColorInterface>::ColorType {
        self.color
    }

    fn remove_key(&mut self, key: Key) {
        if let Some(index) = self.key_map.get(key) {
            self.keys.remove(index);
            self.key_map.remove(key);
            self.count -= 1;
        }
    }

    fn keys_empty(&self) -> bool {
        self.keys.is_empty()
    }
}
