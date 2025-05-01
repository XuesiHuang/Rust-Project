//! Graph module: builds an undirected graph of street intersections and records street names per node.

use crate::io::Segment;
use petgraph::graph::{UnGraph, NodeIndex};
use std::collections::{HashMap, HashSet};

/// `Graph` is an undirected road network where each node stores its (longitude, latitude)
/// and each edge represents a direct street segment between two intersections.
pub type Graph = UnGraph<(f64, f64), ()>;

/// Constructs the road-intersection graph and a mapping of each node to its incident street names.
///
/// Inputs:
/// `segments` - slice of `Segment`, each with a street name and start/end coordinates
///
/// Outputs:
/// `(Graph, HashMap<NodeIndex, HashSet<String>>)`: the constructed graph and a node-to-street-names map
///
/// High-level logic:
/// 1. Initialize an empty undirected graph and two hash maps:
///    `coord_to_node` to coalesce coordinates into unique nodes with 6-decimal rounding
///    `node_streets` to track street names touching each node
/// 2. For each segment:
///    a. Generate string keys for start and end by rounding coordinates to avoid floating-point hash issues
///    b. Use or insert nodes in the graph for those keys
///    c. Add an undirected edge between the two node indices
///    d. Record the segment's street name in `node_streets` for both endpoints
pub fn build_graph(
    segments: &[Segment],
) -> (Graph, HashMap<NodeIndex, HashSet<String>>) {
    // Create an empty undirected graph
    let mut graph = Graph::new_undirected();
    // Map from rounded coordinate string -> node index for de-duplication
    let mut coord_to_node: HashMap<String, NodeIndex> = HashMap::new();
    // Map from node index -> set of street names incident at that node
    let mut node_streets: HashMap<NodeIndex, HashSet<String>> = HashMap::new();

    for seg in segments {
        // a) Round coordinates to 6 decimal places and format as string keys
        let key_a = format!("{:.6},{:.6}", seg.start.0, seg.start.1);
        let key_b = format!("{:.6},{:.6}", seg.end.0,   seg.end.1);
        
        // b) Get or insert the start node
        let a = *coord_to_node
            .entry(key_a)
            .or_insert_with(|| graph.add_node(seg.start));
        // b) Get or insert the end node
        let b = *coord_to_node
            .entry(key_b)
            .or_insert_with(|| graph.add_node(seg.end));
        
        // c) Add an undirected edge between the two nodes
        graph.add_edge(a, b, ());

        // d) Record the street name for both endpoints
        node_streets.entry(a).or_default().insert(seg.name.clone());
        node_streets.entry(b).or_default().insert(seg.name.clone());
    }

    // Return the completed graph and the mapping of node->street names
    (graph, node_streets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    /// Verifies that a simple two-segment chain yields three nodes, two edges,
    /// and that the shared junction contains both street names.
    #[test]
    fn test_build_graph_simple() {
        let segments = vec![
            Segment { name: "A".into(), start: (0.0,0.0), end: (1.0,0.0) },
            Segment { name: "B".into(), start: (1.0,0.0), end: (2.0,0.0) },
        ];
        let (graph, map) = build_graph(&segments);
        // Expect exactly 3 unique nodes and 2 connecting edges
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        
        // The central node at (1.0,0.0) should have both "A" and "B" recorded
        let center = graph.node_indices().find(|&n| graph[n] == (1.0, 0.0)).unwrap();
        let streets = map.get(&center).unwrap();
        assert_eq!(HashSet::from_iter(streets.iter().cloned()), 
                   ["A".into(), "B".into()].into());
    }
}
