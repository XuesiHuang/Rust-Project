//! Centrality module: computes degree, betweenness, and closeness centralities on the road-intersection graph, and prints only true multi-street intersections.

use crate::graph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::algo::dijkstra;
use std::collections::{HashMap, HashSet};

/// Compute the degree centrality for each node in the graph.
///
/// Degree centrality of node v is defined as:
///    deg(v) / (n - 1)
/// where deg(v) is the number of neighbors of v and n is the total number of nodes.
///
/// Inputs:
/// `g`: reference to the road graph (`Graph`)
///
/// Outputs:
/// `HashMap<NodeIndex, f64>` mapping each node to its degree centrality score (0.0–1.0)
pub fn degree_centrality(g: &Graph) -> HashMap<NodeIndex, f64> {
    let n = g.node_count() as f64; // total number of nodes
    let mut scores = HashMap::new();
    for v in g.node_indices() {
        // count number of incident edges (neighbors)
        let deg = g.neighbors(v).count() as f64;
        scores.insert(v, deg / (n - 1.0)); // normalize by max possible degree
    }
    scores
}

/// Compute betweenness centrality for each node using Brandes’ algorithm (unweighted).
///
/// Betweenness centrality of v is the sum, over all pairs s,t, of the fraction of shortest paths from s to t that pass through v.
///
/// Inputs:
/// `g`: reference to the road graph
///
/// Outputs:
/// `HashMap<NodeIndex, f64>` where each entry is the betweenness score for a node
pub fn betweenness_centrality(g: &Graph) -> HashMap<NodeIndex, f64> {
    let mut bc = HashMap::new();
    // initialize score 0 for every node
    for v in g.node_indices() { bc.insert(v, 0.0); }
    let nodes: Vec<_> = g.node_indices().collect(); // list of all nodes

    // for each source node s
    for &s in &nodes {
        // stack to record order of BFS
        let mut stack = Vec::new();
        // predecessors list: for each v, store list of preceding nodes on shortest paths
        let mut preds = nodes.iter().map(|&v| (v, Vec::new())).collect::<HashMap<_, _>>();
        // sigma: number of shortest paths from s to v
        let mut sigma = nodes.iter().map(|&v| (v, 0.0)).collect::<HashMap<_, _>>();
        // distance from s to v (-1 indicates unreachable)
        let mut dist = nodes.iter().map(|&v| (v, -1)).collect::<HashMap<_, _>>();
        sigma.insert(s, 1.0);
        dist.insert(s, 0);

        // BFS from s to compute distances and sigma
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(s);
        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for w in g.neighbors(v) {
                if dist[&w] < 0 {
                    // first time visited => set distance and enqueue
                    dist.insert(w, dist[&v] + 1);
                    queue.push_back(w);
                }
                if dist[&w] == dist[&v] + 1 {
                    // found shortest path to w via v => update sigma and preds
                    sigma.insert(w, sigma[&w] + sigma[&v]);
                    preds.get_mut(&w).unwrap().push(v);
                }
            }
        }

        // accumulate dependencies
        let mut delta = nodes.iter().map(|&v| (v, 0.0)).collect::<HashMap<_, _>>();
        while let Some(w) = stack.pop() {
            // for each predecessor v of w
            for &v in &preds[&w] {
                let c = (sigma[&v] / sigma[&w]) * (1.0 + delta[&w]);
                *delta.get_mut(&v).unwrap() += c; // accumulate dependency
            }
            if w != s {
                // do not count source itself
                *bc.get_mut(&w).unwrap() += delta[&w];
            }
        }
    }

    // for undirected graph, divide by 2 to account for double-counting
    for val in bc.values_mut() { *val /= 2.0; }
    bc
}

/// Compute closeness centrality for each node.
///
/// Closeness of v is defined as:
///    (reachable_count - 1) / sum_of_distances_to_reachable_nodes where `reachable_count` includes v itself.
///
/// Inputs：
/// `g`: reference to the road graph
///
/// Outputs：
/// `HashMap<NodeIndex, f64>` mapping node -> closeness centrality score
pub fn closeness_centrality(g: &Graph) -> HashMap<NodeIndex, f64> {
    let mut scores = HashMap::new();
    for v in g.node_indices() {
        // compute shortest-path distances from v to all reachable nodes
        let paths = dijkstra(g, v, None, |_| 1.0);
        let total: f64 = paths.values().sum(); // sum of distances
        let reach = paths.len() as f64;       // number of reachable nodes
        let c = if total > 0.0 { (reach - 1.0) / total } else { 0.0 };
        scores.insert(v, c);
    }
    scores
}

/// Print the top-K intersections by centrality score, excluding dead-ends and single-street nodes.
///
/// Inputs：
/// `title`: label printed before the list
/// `scores`: centrality scores map (node -> score)
/// `k`: number of top entries to print
/// `graph`: reference to the road graph for accessing node coordinates
/// `node_streets`: map from node to set of street names
pub fn print_top_with_names(
    title: &str,
    scores: &HashMap<NodeIndex, f64>,
    k: usize,
    graph: &Graph,
    node_streets: &HashMap<NodeIndex, HashSet<String>>,
) {
    println!("--- {} ---", title);
    // Build list of (node,score), filtering only true intersections:
    let mut vec: Vec<_> = scores
        .iter()
        .filter(|(&node, _)| {
            // skip graph leaves (degree == 1)
            graph.neighbors(node).count() > 1
            // skip nodes without multiple distinct street names
            && node_streets.get(&node).map_or(false, |hs| hs.len() > 1)
        })
        .collect();

    // Sort by descending score
    vec.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    // Print top k entries with node index, coordinates, score, and street list
    for (&node, &score) in vec.into_iter().take(k) {
        let (lon, lat) = graph[node];
        let names = node_streets[&node]
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(" & ");
        println!(
            "Node {} @ ({:.6},{:.6}): {:>8.5}   Streets: {}",
            node.index(), lon, lat, score, names
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::Segment;
    use crate::graph::build_graph;

    /// Test centrality on a simple 3-node line graph A—B—C, where B is the middle.
    #[test]
    fn test_degree_and_closeness_line() {
        let segments = vec![
            Segment { name: "X".into(), start: (0.0,0.0), end: (1.0,0.0) },
            Segment { name: "X".into(), start: (1.0,0.0), end: (2.0,0.0) },
        ];
        let (g, _) = build_graph(&segments);
        let dc = degree_centrality(&g);
        // Middle node has degree 2 of 2 possible => normalized 1.0
        assert!((dc[&NodeIndex::new(1)] - 1.0).abs() < 1e-6);
        let cc = closeness_centrality(&g);
        // Middle node reaches 3 nodes with sum distance 2 => (3-1)/2 = 1.0
        assert!((cc[&NodeIndex::new(1)] - 1.0).abs() < 1e-6);
    }
}
