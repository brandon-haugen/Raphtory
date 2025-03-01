//! Local Clustering coefficient - measures the degree to which nodes in a graph tend to cluster together.
//!
//! It is calculated by dividing the number of triangles (sets of three nodes that are all
//! connected to each other) in the graph by the total number of possible triangles.
//! The resulting value is a number between 0 and 1 that represents the density of
//! clustering in the graph.
//!
//! A high clustering coefficient indicates that nodes tend to be
//! connected to nodes that are themselves connected to each other, while a low clustering
//! coefficient indicates that nodes tend to be connected to nodes that are not connected
//! to each other.
//!
//! In a social network of a particular community_detection, we can compute the clustering
//! coefficient of each node to get an idea of how strongly connected and cohesive
//! that node's neighborhood is.
//!
//! A high clustering coefficient for a node in a social network indicates that the
//! node's neighbors tend to be strongly connected with each other, forming a tightly-knit
//! group or community_detection. In contrast, a low clustering coefficient for a node indicates that
//! its neighbors are relatively less connected with each other, suggesting a more fragmented
//! or diverse community_detection.
//!
//! # Examples
//!
//! ```rust
//! use raphtory::algorithms::metrics::local_clustering_coefficient::local_clustering_coefficient;
//! use raphtory::prelude::*;
//!
//! let g = Graph::new();
//! let windowed_graph = g.window(0, 7);
//! let vs = vec![
//!     (1, 1, 2),
//!     (2, 1, 3),
//!     (3, 2, 1),
//!     (4, 3, 2),
//!     (5, 1, 4),
//!     (6, 4, 5),
//! ];
//!
//! for (t, src, dst) in &vs {
//!     g.add_edge(*t, *src, *dst, NO_PROPS, None);
//! }
//!
//! let actual = (1..=5)
//! .map(|v| local_clustering_coefficient(&windowed_graph, v))
//! .collect::<Vec<_>>();
//!
//! println!("local clustering coefficient of all nodes: {:?}", actual);
//! ```

use crate::{
    algorithms::motifs::local_triangle_count::local_triangle_count,
    core::entities::nodes::node_ref::AsNodeRef, db::api::view::*,
};

/// measures the degree to which nodes in a graph tend to cluster together
pub fn local_clustering_coefficient<G: StaticGraphViewOps, V: AsNodeRef>(
    graph: &G,
    v: V,
) -> Option<f32> {
    let v = v.as_node_ref();
    if let Some(node) = graph.node(v) {
        if let Some(triangle_count) = local_triangle_count(graph, v) {
            let triangle_count = triangle_count as f32;
            let degree = node.degree() as f32;
            if degree > 1.0 {
                Some((2.0 * triangle_count) / (degree * (degree - 1.0)))
            } else {
                Some(0.0)
            }
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod clustering_coefficient_tests {
    use super::local_clustering_coefficient;
    use crate::{
        db::{
            api::{mutation::AdditionOps, view::*},
            graph::graph::Graph,
        },
        prelude::NO_PROPS,
        test_storage,
    };

    #[test]
    fn clusters_of_triangles() {
        let graph = Graph::new();
        let vs = vec![
            (1, 1, 2),
            (2, 1, 3),
            (3, 2, 1),
            (4, 3, 2),
            (5, 1, 4),
            (6, 4, 5),
        ];

        for (t, src, dst) in &vs {
            graph.add_edge(*t, *src, *dst, NO_PROPS, None).unwrap();
        }

        test_storage!(&graph, |graph| {
            let expected = vec![0.33333334, 1.0, 1.0, 0.0, 0.0];
            let windowed_graph = graph.window(0, 7);
            let actual = (1..=5)
                .map(|v| local_clustering_coefficient(&windowed_graph, v).unwrap())
                .collect::<Vec<_>>();

            assert_eq!(actual, expected);
        });
    }
}
