use crate::node_impl::NodeView;
use soroban_sdk::contracttype;

#[derive(Clone, Copy)]
#[contracttype]
pub struct Tree {
    pub(crate) root: NodeView,
}
