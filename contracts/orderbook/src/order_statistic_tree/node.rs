use crate::error::Error;
use core::fmt::Debug;
use core::ops::Deref;
pub trait ColorInterface {
    type ColorType: PartialEq + Copy + Clone + Debug;

    fn black() -> Self::ColorType;
    fn red() -> Self::ColorType;
}

pub trait NodeInterface<
    NodeViewT: Clone + core::fmt::Debug + Copy + PartialEq + PartialOrd + ColorInterface,
>
{
    fn right_mut(&mut self) -> &mut NodeViewT;

    fn left_mut(&mut self) -> &mut NodeViewT;

    fn parent_mut(&mut self) -> &mut NodeViewT;

    fn color_mut(&mut self) -> &mut NodeViewT::ColorType;

    fn left(&self) -> NodeViewT;

    fn right(&self) -> NodeViewT;

    fn parent(&self) -> NodeViewT;

    fn color(&self) -> NodeViewT::ColorType;

    fn key_exists(&self, key: u64) -> bool;

    fn insert_key(&mut self, key: u64);

    fn remove_key(&mut self, key: u64);

    fn keys_empty(&self) -> bool;
}

pub trait NodeViewInterface {
    fn is_empty(&self) -> bool;
    fn new(id: u128) -> Self;
    fn empty_node() -> Self;
    fn nil_node() -> Self;
    fn to_raw(&self) -> Option<u128>;
}

pub trait StorageAccessor: Sized + Copy {
    type NodeViewT: Clone
        + core::fmt::Debug
        + Copy
        + PartialEq
        + PartialOrd
        + NodeViewInterface
        + ColorInterface;
    type InnerNodeT: NodeInterface<Self::NodeViewT>;

    fn load(&self, node_view: Self::NodeViewT) -> Result<InMemoryNode<Self>, Error>;
    fn upload(&self, node_view: Self::NodeViewT, in_node: &Self::InnerNodeT) -> Result<(), Error>;
    fn new_node(
        &self,
        parent: Self::NodeViewT,
        id: Self::NodeViewT,
        key: u64,
    ) -> Result<NodeViewHolder<Self>, Error>;
    fn remove_node(&self, node_view: Self::NodeViewT) -> Result<(), Error>;
    fn to_node_holder(&self, node_view: Self::NodeViewT) -> NodeViewHolder<Self>;
    fn node_exists(&self, node_view: Self::NodeViewT) -> bool;
    fn create_nil_node(&self, parent: Self::NodeViewT) -> Result<NodeViewHolder<Self>, Error>;
}

#[derive(Copy, Clone, Eq)]
pub struct NodeViewHolder<T: StorageAccessor> {
    storage_accessor: T,
    node_view: T::NodeViewT,
}

impl<T: StorageAccessor> PartialEq for NodeViewHolder<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node_view == other.node_view
    }
}

impl<T: StorageAccessor> Deref for NodeViewHolder<T> {
    type Target = T::NodeViewT;

    fn deref(&self) -> &Self::Target {
        &self.node_view
    }
}

impl<T: StorageAccessor> NodeViewHolder<T> {
    pub fn new(node_view: T::NodeViewT, storage_accessor: T) -> Self {
        Self {
            node_view,
            storage_accessor,
        }
    }

    pub fn get_view(&self) -> T::NodeViewT {
        self.node_view
    }
}

pub struct InMemoryNode<T: StorageAccessor> {
    pub is_modified: bool,
    pub id: T::NodeViewT,
    pub inner_node: T::InnerNodeT,
    pub storage_accessor: T,
}

impl<T: StorageAccessor> Drop for InMemoryNode<T> {
    fn drop(&mut self) {
        if self.is_modified && !self.keys_empty() {
            // the keys_empty check required in order to not insert the empty node once again
            self.storage_accessor
                .upload(self.id, &self.inner_node)
                .expect("Error dropping in memory node with id");
        }
    }
}

impl<T: StorageAccessor + Copy> Deref for InMemoryNode<T> {
    type Target = T::NodeViewT;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<T: StorageAccessor> InMemoryNode<T> {
    pub fn node_view(&self) -> NodeViewHolder<T> {
        NodeViewHolder {
            storage_accessor: self.storage_accessor,
            node_view: self.id,
        }
    }

    pub fn color(&self) -> <<T as StorageAccessor>::NodeViewT as ColorInterface>::ColorType {
        self.inner_node.color()
    }

    pub fn color_mut(
        &mut self,
    ) -> &mut <<T as StorageAccessor>::NodeViewT as ColorInterface>::ColorType {
        self.is_modified = true;
        self.inner_node.color_mut()
    }

    pub fn right_mut(&mut self) -> &mut T::NodeViewT {
        self.is_modified = true;
        self.inner_node.right_mut()
    }

    pub fn left_mut(&mut self) -> &mut T::NodeViewT {
        self.is_modified = true;
        self.inner_node.left_mut()
    }

    pub fn parent_mut(&mut self) -> &mut T::NodeViewT {
        self.is_modified = true;
        self.inner_node.parent_mut()
    }

    pub fn left(&self) -> NodeViewHolder<T> {
        NodeViewHolder {
            storage_accessor: self.storage_accessor,
            node_view: self.inner_node.left(),
        }
    }

    pub fn right(&self) -> NodeViewHolder<T> {
        NodeViewHolder {
            storage_accessor: self.storage_accessor,
            node_view: self.inner_node.right(),
        }
    }

    pub fn parent(&self) -> NodeViewHolder<T> {
        NodeViewHolder {
            storage_accessor: self.storage_accessor,
            node_view: self.inner_node.parent(),
        }
    }

    pub fn key_exists(&self, key: u64) -> bool {
        self.inner_node.key_exists(key)
    }

    pub fn insert_key(&mut self, key: u64) {
        self.is_modified = true;
        self.inner_node.insert_key(key);
    }

    pub fn remove_key(&mut self, key: u64) {
        self.is_modified = true;
        self.inner_node.remove_key(key);
    }

    pub fn keys_empty(&self) -> bool {
        self.inner_node.keys_empty()
    }
}

impl<T: StorageAccessor> InMemoryNode<T> {
    pub fn sync(self) -> Result<NodeViewHolder<T>, Error> {
        let view = self.id;
        self.storage_accessor.upload(view, &self.inner_node)?;

        Ok(NodeViewHolder::new(self.id, self.storage_accessor))
    }
}

impl<T: StorageAccessor> NodeViewHolder<T> {
    pub fn is_empty(&self) -> bool {
        self.node_view.is_empty()
    }

    pub fn load(self) -> Result<InMemoryNode<T>, Error> {
        self.storage_accessor.load(self.node_view)
    }
}
