//! Node registry for named node lookup.
//!
//! Provides a registry for storing and retrieving node functions by name.

use std::collections::HashMap;
use std::marker::PhantomData;

use crate::error::GraphError;
use crate::graph::state::{NodeFn, State};

/// A registry of named node functions, parameterised by state type.
///
/// Allows nodes to be registered ahead of time and referenced by name
/// when building graphs.
pub struct NodeRegistry<S: State> {
    nodes: HashMap<String, NodeFn<S>>,
    _marker: PhantomData<S>,
}

impl<S: State> NodeRegistry<S> {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            _marker: PhantomData,
        }
    }

    /// Registers a node function with the given name.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DuplicateNode`] if a node with the same name
    /// already exists.
    pub fn register(&mut self, name: impl Into<String>, func: NodeFn<S>) -> Result<(), GraphError> {
        let name = name.into();
        if self.nodes.contains_key(&name) {
            return Err(GraphError::DuplicateNode { name });
        }
        let _prev = self.nodes.insert(name, func);
        Ok(())
    }

    /// Removes and returns the node function with the given name.
    pub fn take(&mut self, name: &str) -> Option<NodeFn<S>> {
        self.nodes.remove(name)
    }

    /// Returns the number of registered nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl<S: State> Default for NodeRegistry<S> {
    fn default() -> Self {
        Self::new()
    }
}
