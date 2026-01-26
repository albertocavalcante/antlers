//! Dependency graph for visualization and analysis.
//!
//! This module provides [`DependencyGraph`] for representing the dependency
//! tree structure, useful for visualization and conflict analysis.

use std::collections::HashMap;

use gav::{Artifact, Coordinates};

/// A node in the dependency graph.
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// The artifact at this node.
    pub artifact: Artifact,
    /// The depth of this node in the tree (root = 0).
    pub depth: usize,
}

impl GraphNode {
    /// Creates a new graph node.
    #[must_use]
    pub const fn new(artifact: Artifact, depth: usize) -> Self {
        Self { artifact, depth }
    }

    /// Returns the coordinates for this node.
    #[must_use]
    pub fn coordinates(&self) -> Coordinates {
        self.artifact.coordinates.clone()
    }
}

/// A dependency graph for visualization and analysis.
///
/// This graph structure tracks the dependency relationships and can be used
/// for visualization, conflict analysis, and understanding the dependency tree.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Nodes in the graph, indexed by coordinates.
    nodes: HashMap<Coordinates, GraphNode>,
    /// Edges in the graph (from -> to).
    edges: Vec<(Coordinates, Coordinates)>,
}

impl DependencyGraph {
    /// Creates a new empty dependency graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Adds a node to the graph.
    ///
    /// If a node with the same coordinates already exists, it is replaced.
    pub fn add_node(&mut self, artifact: Artifact, depth: usize) {
        let coords = artifact.coordinates.clone();
        self.nodes.insert(coords, GraphNode::new(artifact, depth));
    }

    /// Adds an edge to the graph.
    ///
    /// The edge represents a dependency from `from` to `to`.
    pub fn add_edge(&mut self, from: &Coordinates, to: &Coordinates) {
        self.edges.push((from.clone(), to.clone()));
    }

    /// Returns the node for the given coordinates.
    #[must_use]
    pub fn get_node(&self, coords: &Coordinates) -> Option<&GraphNode> {
        self.nodes.get(coords)
    }

    /// Returns all nodes in the graph.
    #[must_use]
    pub const fn nodes(&self) -> &HashMap<Coordinates, GraphNode> {
        &self.nodes
    }

    /// Returns all edges in the graph.
    #[must_use]
    pub fn edges(&self) -> &[(Coordinates, Coordinates)] {
        &self.edges
    }

    /// Returns the root nodes (nodes with no incoming edges).
    #[must_use]
    pub fn roots(&self) -> Vec<&Coordinates> {
        let targets: std::collections::HashSet<_> = self.edges.iter().map(|(_, to)| to).collect();

        self.nodes
            .keys()
            .filter(|coords| !targets.contains(coords))
            .collect()
    }

    /// Returns the children of a node (nodes that this node depends on).
    #[must_use]
    pub fn children(&self, node: &Coordinates) -> Vec<&Coordinates> {
        self.edges
            .iter()
            .filter_map(|(from, to)| if from == node { Some(to) } else { None })
            .collect()
    }

    /// Returns the parents of a node (nodes that depend on this node).
    #[must_use]
    pub fn parents(&self, node: &Coordinates) -> Vec<&Coordinates> {
        self.edges
            .iter()
            .filter_map(|(from, to)| if to == node { Some(from) } else { None })
            .collect()
    }

    /// Returns the total number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the total number of edges.
    #[must_use]
    pub const fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns true if the graph is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns all nodes at a given depth.
    #[must_use]
    pub fn nodes_at_depth(&self, depth: usize) -> Vec<&GraphNode> {
        self.nodes
            .values()
            .filter(|node| node.depth == depth)
            .collect()
    }

    /// Returns the maximum depth in the graph.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.nodes
            .values()
            .map(|node| node.depth)
            .max()
            .unwrap_or(0)
    }

    /// Checks if there is a path from `from` to `to`.
    ///
    /// This can be used to detect cycles if `from == to` after traversal.
    #[must_use]
    pub fn has_path(&self, from: &Coordinates, to: &Coordinates) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if current == to {
                return true;
            }

            if visited.insert(current) {
                for child in self.children(current) {
                    stack.push(child);
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(group: &str, name: &str, version: &str) -> Artifact {
        Artifact::new(group, name, version)
    }

    fn coords(group: &str, name: &str) -> Coordinates {
        Coordinates::new(group, name)
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_nodes() {
        let mut graph = DependencyGraph::new();
        graph.add_node(artifact("com.example", "a", "1.0"), 0);
        graph.add_node(artifact("com.example", "b", "2.0"), 1);

        assert_eq!(graph.node_count(), 2);
        assert!(!graph.is_empty());

        let a = graph.get_node(&coords("com.example", "a")).unwrap();
        assert_eq!(a.depth, 0);
        assert_eq!(a.artifact.version.as_str(), "1.0");
    }

    #[test]
    fn test_add_edges() {
        let mut graph = DependencyGraph::new();
        let a_coords = coords("com.example", "a");
        let b_coords = coords("com.example", "b");

        graph.add_node(artifact("com.example", "a", "1.0"), 0);
        graph.add_node(artifact("com.example", "b", "2.0"), 1);
        graph.add_edge(&a_coords, &b_coords);

        assert_eq!(graph.edge_count(), 1);

        let children = graph.children(&a_coords);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], &b_coords);

        let parents = graph.parents(&b_coords);
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0], &a_coords);
    }

    #[test]
    fn test_roots() {
        let mut graph = DependencyGraph::new();
        let a_coords = coords("com.example", "a");
        let b_coords = coords("com.example", "b");
        let c_coords = coords("com.example", "c");

        graph.add_node(artifact("com.example", "a", "1.0"), 0);
        graph.add_node(artifact("com.example", "b", "2.0"), 1);
        graph.add_node(artifact("com.example", "c", "3.0"), 1);
        graph.add_edge(&a_coords, &b_coords);
        graph.add_edge(&a_coords, &c_coords);

        let roots = graph.roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], &a_coords);
    }

    #[test]
    fn test_has_path() {
        let mut graph = DependencyGraph::new();
        let a_coords = coords("com.example", "a");
        let b_coords = coords("com.example", "b");
        let c_coords = coords("com.example", "c");

        graph.add_node(artifact("com.example", "a", "1.0"), 0);
        graph.add_node(artifact("com.example", "b", "2.0"), 1);
        graph.add_node(artifact("com.example", "c", "3.0"), 2);
        graph.add_edge(&a_coords, &b_coords);
        graph.add_edge(&b_coords, &c_coords);

        assert!(graph.has_path(&a_coords, &c_coords));
        assert!(!graph.has_path(&c_coords, &a_coords));
    }

    #[test]
    fn test_nodes_at_depth() {
        let mut graph = DependencyGraph::new();
        graph.add_node(artifact("com.example", "a", "1.0"), 0);
        graph.add_node(artifact("com.example", "b", "2.0"), 1);
        graph.add_node(artifact("com.example", "c", "3.0"), 1);
        graph.add_node(artifact("com.example", "d", "4.0"), 2);

        let depth_0 = graph.nodes_at_depth(0);
        assert_eq!(depth_0.len(), 1);

        let depth_1 = graph.nodes_at_depth(1);
        assert_eq!(depth_1.len(), 2);

        let depth_2 = graph.nodes_at_depth(2);
        assert_eq!(depth_2.len(), 1);

        assert_eq!(graph.max_depth(), 2);
    }
}
