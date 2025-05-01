//! Main module: orchestrates CLI parsing, data loading, graph construction, centrality computation, and result printing.

mod io;
mod graph;
mod centrality;

use std::path::PathBuf;
use structopt::StructOpt;

/// `Opt` represents the command-line options for this program.
///
/// Fields:
/// `path` - the filesystem path to the CSV file containing road segment data.
#[derive(StructOpt)]
struct Opt {
    /// Path to the managed-streets CSV file
    #[structopt(parse(from_os_str))]
    path: PathBuf,
}

/// Entry point of the application.
///
/// Returns:
/// `Ok(())` on success
/// `Err` if any I/O or parsing error occurs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the CLI arguments into `Opt`
    let opt = Opt::from_args();

    // Load all road segments from the CSV file at the given path
    // Returns Vec<io::Segment> or an error if parsing fails
    let segments = io::load_segments(&opt.path)?;

    // Build an undirected graph of intersections and a map of node->street names
    // `graph` holds topology; `node_streets` associates each node with its street names
    let (graph, node_streets) = graph::build_graph(&segments);

    // Compute three centrality measures on the graph
    // 1) Degree centrality: fraction of possible edges present
    // 2) Betweenness centrality: frequency on shortest paths
    // 3) Closeness centrality: inverse of average distance to all reachable nodes
    let deg = centrality::degree_centrality(&graph);
    let btw = centrality::betweenness_centrality(&graph);
    let cls = centrality::closeness_centrality(&graph);

    // Print the top 20 intersections by each centrality measure
    // Only true intersections (>=2 streets) are displayed
    centrality::print_top_with_names(
        "Degree Centrality",
        &deg,
        20,
        &graph,
        &node_streets,
    );
    centrality::print_top_with_names(
        "Betweenness Centrality",
        &btw,
        20,
        &graph,
        &node_streets,
    );
    centrality::print_top_with_names(
        "Closeness Centrality",
        &cls,
        20,
        &graph,
        &node_streets,
    );

    Ok(())
}
