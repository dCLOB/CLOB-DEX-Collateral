## OrderStatistic Tree - README

## Introduction

An **OrderStatistic Tree** is a special type of binary search tree (BST) that efficiently supports two key operations:
1. **Inserting/Deleting** a key while maintaining order.
2. **Finding the k-th smallest (or largest) element** in the tree in logarithmic time.

The OrderStatistic Tree extends the typical binary search tree by keeping track of the number of elements in each subtree, which allows for quick queries regarding the rank of an element or the element corresponding to a particular rank.

OrderStatistic Trees are often implemented as **self-balancing trees**, such as **Red-Black Trees**, to ensure efficient performance even in the worst-case scenarios. This specific implementation utilizes a Red-Black Tree structure combined with node size tracking for rank and selection operations.

---

## Core Concepts

### Binary Search Tree (BST)
- A **Binary Search Tree (BST)** is a tree data structure where each node has at most two children, referred to as the left and right children.
- For each node:
  - All values in the left subtree are smaller than the node’s value.
  - All values in the right subtree are larger than the node’s value.

### Red-Black Tree
- A **Red-Black Tree** is a type of self-balancing binary search tree that maintains balance by assigning a color (red or black) to each node.
- It guarantees that the tree’s height is logarithmic with respect to the number of nodes, ensuring that operations such as insertion, deletion, and searching can be performed in **O(log n)** time.

### OrderStatistic Tree (OST)
- An **OrderStatistic Tree** is a Red-Black Tree with an additional feature: it tracks the size (i.e., number of nodes) of each subtree.
- This extra information allows for two additional operations:
  - **Rank Queries**: Find the rank of a given element (i.e., how many elements are smaller than or equal to the given element).
  - **Select Queries**: Find the k-th smallest (or largest) element in the tree.

### Nodes in an OrderStatistic Tree
Each node in the OrderStatistic Tree contains:
- **Key**: The value stored in the node.
- **Color**: Red or Black (in Red-Black Trees).
- **Left**: Pointer to the left child.
- **Right**: Pointer to the right child.
- **Parent**: Pointer to the parent node.
- **Subtree Size**: The total number of nodes in the subtree rooted at this node (including itself).

### Rank and Selection Operations

1. **Rank of a Node**:
   - The rank of a node is its position in an **in-order traversal** of the tree. 
   - For example, the rank of the minimum element is 1, and the rank of the maximum element is the total number of nodes in the tree.
   - By using the subtree size information, the rank of any node can be computed efficiently.

2. **Select Operation**:
   - The select operation finds the k-th smallest element in the tree. 
   - The subtree sizes help guide the search: if the left subtree contains fewer than k nodes, we know the k-th element must lie in the right subtree or be the current node.

---

## Operations Overview

### Insert
Insertion into an OrderStatistic Tree works similarly to insertion into a standard Red-Black Tree, but after the insertion, the subtree sizes need to be updated. The tree remains balanced, and the insertion takes **O(log n)** time.

### Delete
Deletion from an OrderStatistic Tree involves standard deletion from a Red-Black Tree, with additional steps to maintain subtree sizes and rebalance the tree. The deletion operation takes **O(log n)** time.

### Rank Query
The rank of a given node is computed based on the sizes of the left subtree and parent nodes. This operation can be completed in **O(log n)** time.

### Select Query
The select operation involves traversing the tree based on subtree sizes to find the k-th smallest element. It can be performed in **O(log n)** time.

---

### Order Statistic tree in Soroban
For the Soroban framework, the tree can be maintained within a smart contract's state, which allows for efficient querying and updates. Since Soroban is designed for on-chain computations, storage and performance considerations must be handled carefully. The tree operations (insertion, deletion, and rank queries) can be defined as contract functions, allowing for interaction through Soroban's native API.

One key challenge is ensuring that the tree remains balanced and efficient when executed in a decentralized context.
In summary, integrating the Order Statistic Tree with Soroban allows developers to maintain dynamic sorted data structures within smart contracts, which could be effectively used for the onchain orderbook implementation enabling powerful querying capabilities directly on the blockchain.

While implementing the order statistic tree using the Soroban framework should be carefully taken into account the specific storage management approach. More details on this could be found [here](https://developers.stellar.org/docs/learn/encyclopedia/storage/persisting-data).

The general structure of the Order Statistic tree in Soroban could be represented with the tree structure which would store the root node id and storage of all nodes.

```rust
#[contracttype]
#[derive(Debug, Default)]
struct Tree {
    root: Option<u64>,
    nodes: HashMap<u64, Node>,
}
```

The Node structure would contain the next fields:

```rust
#[contracttype]
#[derive(Debug, Default)]
struct Node {
    parent: Option<u64>,
    left: Option<u64>,
    right: Option<u64>,
    red: bool,
    keys: Vec<u64>,
    key_map: HashMap<u64, usize>,
    count: u64,
}
```

---

## Summary

An OrderStatistic Tree is a powerful data structure for dynamic ordered sets, combining the self-balancing properties of Red-Black Trees with additional capabilities for rank and selection operations. This allows efficient handling of real-time data where order matters and quick rank-based queries are essential.
