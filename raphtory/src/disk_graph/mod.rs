pub mod graph_impl;
pub mod query;
pub mod storage_interface;

pub type Time = i64;

pub mod prelude {
    pub use pometry_storage::chunked_array::array_ops::*;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Raphtory Arrow Error: {0}")]
    RAError(#[from] pometry_storage::RAError),
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{
        arrow2::datatypes::{ArrowDataType as DataType, ArrowSchema as Schema},
        db::graph::graph::assert_graph_equal,
        prelude::*,
    };
    use itertools::Itertools;
    use polars_arrow::{
        array::{PrimitiveArray, StructArray},
        datatypes::Field,
    };
    use pometry_storage::{global_order::GlobalMap, graph_fragment::TempColGraphFragment, RAError};
    use proptest::{prelude::*, sample::size_range};
    use raphtory_api::core::{
        entities::{EID, VID},
        Direction,
    };
    use tempfile::TempDir;

    fn edges_sanity_node_list(edges: &[(u64, u64, i64)]) -> Vec<u64> {
        edges
            .iter()
            .map(|(s, _, _)| *s)
            .chain(edges.iter().map(|(_, d, _)| *d))
            .sorted()
            .dedup()
            .collect()
    }

    pub fn edges_sanity_check_build_graph<P: AsRef<Path>>(
        test_dir: P,
        edges: &[(u64, u64, i64)],
        nodes: &[u64],
        input_chunk_size: u64,
        chunk_size: usize,
        t_props_chunk_size: usize,
    ) -> Result<TempColGraphFragment, RAError> {
        let chunks = edges
            .iter()
            .map(|(src, _, _)| *src)
            .chunks(input_chunk_size as usize);
        let srcs = chunks
            .into_iter()
            .map(|chunk| PrimitiveArray::from_vec(chunk.collect()));
        let chunks = edges
            .iter()
            .map(|(_, dst, _)| *dst)
            .chunks(input_chunk_size as usize);
        let dsts = chunks
            .into_iter()
            .map(|chunk| PrimitiveArray::from_vec(chunk.collect()));
        let chunks = edges
            .iter()
            .map(|(_, _, times)| *times)
            .chunks(input_chunk_size as usize);
        let times = chunks
            .into_iter()
            .map(|chunk| PrimitiveArray::from_vec(chunk.collect()));

        let schema = Schema::from(vec![
            Field::new("srcs", DataType::UInt64, false),
            Field::new("dsts", DataType::UInt64, false),
            Field::new("time", DataType::Int64, false),
        ]);

        let triples = srcs.zip(dsts).zip(times).map(move |((a, b), c)| {
            StructArray::new(
                DataType::Struct(schema.fields.clone()),
                vec![a.boxed(), b.boxed(), c.boxed()],
                None,
            )
        });

        let go: GlobalMap = nodes.iter().copied().collect();
        let node_gids = PrimitiveArray::from_slice(nodes).boxed();

        let mut graph = TempColGraphFragment::load_from_edge_list(
            test_dir.as_ref(),
            0,
            chunk_size,
            t_props_chunk_size,
            go.into(),
            node_gids,
            0,
            1,
            2,
            triples,
        )?;
        graph.build_node_additions(chunk_size)?;
        Ok(graph)
    }

    fn check_graph_sanity(edges: &[(u64, u64, i64)], nodes: &[u64], graph: &TempColGraphFragment) {
        let expected_graph = Graph::new();
        for (src, dst, t) in edges {
            expected_graph
                .add_edge(*t, *src, *dst, NO_PROPS, None)
                .unwrap();
        }

        let graph_dir = TempDir::new().unwrap();
        // check persist_as_disk_graph works
        let disk_graph_from_expected = expected_graph
            .persist_as_disk_graph(graph_dir.path())
            .unwrap();
        assert_graph_equal(&disk_graph_from_expected, &expected_graph);

        let actual_num_verts = nodes.len();
        let g_num_verts = graph.num_nodes();
        assert_eq!(actual_num_verts, g_num_verts);
        assert!(graph
            .all_edges_iter()
            .all(|e| e.src().0 < g_num_verts && e.dst().0 < g_num_verts));

        for v in 0..g_num_verts {
            let v = VID(v);
            assert!(graph
                .edges(v, Direction::OUT)
                .map(|(_, v)| v)
                .tuple_windows()
                .all(|(v1, v2)| v1 <= v2));
            assert!(graph
                .edges(v, Direction::IN)
                .map(|(_, v)| v)
                .tuple_windows()
                .all(|(v1, v2)| v1 <= v2));
        }

        let exploded_edges: Vec<_> = graph
            .exploded_edges()
            .map(|e| (nodes[e.src().0], nodes[e.dst().0], e.timestamp()))
            .collect();
        assert_eq!(exploded_edges, edges);

        // check incoming edges
        for (v_id, g_id) in nodes.iter().enumerate() {
            let node = expected_graph.node(*g_id).unwrap();
            let mut expected_inbound = node.in_edges().id().map(|(v, _)| v).collect::<Vec<_>>();
            expected_inbound.sort();

            let actual_inbound = graph
                .edges(VID(v_id), Direction::IN)
                .map(|(_, v)| nodes[v.0])
                .collect::<Vec<_>>();

            assert_eq!(expected_inbound, actual_inbound);
        }

        let unique_edges = edges.iter().map(|(src, dst, _)| (*src, *dst)).dedup();

        for (e_id, (src, dst)) in unique_edges.enumerate() {
            let edge = graph.edge(EID(e_id));
            let VID(src_id) = edge.src();
            let VID(dst_id) = edge.dst();

            assert_eq!(nodes[src_id], src);
            assert_eq!(nodes[dst_id], dst);
        }
    }

    fn edges_sanity_check_inner(
        edges: Vec<(u64, u64, i64)>,
        input_chunk_size: u64,
        chunk_size: usize,
        t_props_chunk_size: usize,
    ) {
        let test_dir = TempDir::new().unwrap();
        let nodes = edges_sanity_node_list(&edges);
        match edges_sanity_check_build_graph(
            test_dir.path(),
            &edges,
            &nodes,
            input_chunk_size,
            chunk_size,
            t_props_chunk_size,
        ) {
            Ok(graph) => {
                // check graph is sane
                check_graph_sanity(&edges, &nodes, &graph);
                let node_gids = PrimitiveArray::from_slice(&nodes).boxed();

                // check that reloading from graph dir works
                let reloaded_graph =
                    TempColGraphFragment::new(test_dir.path(), true, 0, node_gids).unwrap();
                check_graph_sanity(&edges, &nodes, &reloaded_graph)
            }
            Err(RAError::NoEdgeLists | RAError::EmptyChunk) => assert!(edges.is_empty()),
            Err(error) => panic!("{}", error.to_string()),
        };
    }

    proptest! {
        #[test]
        fn edges_sanity_check(
            edges in any_with::<Vec<(u8, u8, Vec<i64>)>>(size_range(1..=100).lift()).prop_map(|v| {
                let mut v: Vec<(u64, u64, i64)> = v.into_iter().flat_map(|(src, dst, times)| {
                    let src = src as u64;
                    let dst = dst as u64;
                    times.into_iter().map(move |t| (src, dst, t))}).collect();
                v.sort();
                v}),
            input_chunk_size in 1..1024u64,
            chunk_size in 1..1024usize,
            t_props_chunk_size in 1..128usize
        ) {
            edges_sanity_check_inner(edges, input_chunk_size, chunk_size, t_props_chunk_size);
        }
    }

    #[test]
    fn edge_sanity_bad() {
        let edges = vec![
            (0, 85, -8744527736816607775),
            (0, 85, -8533859256444633783),
            (0, 85, -7949123054744509169),
            (0, 85, -7208573652910411733),
            (0, 85, -7004677070223473589),
            (0, 85, -6486844751834401685),
            (0, 85, -6420653301843451067),
            (0, 85, -6151481582745013767),
            (0, 85, -5577061971106014565),
            (0, 85, -5484794766797320810),
        ];
        edges_sanity_check_inner(edges, 3, 5, 6)
    }

    #[test]
    fn edge_sanity_more_bad() {
        let edges = vec![
            (1, 3, -8622734205120758463),
            (2, 0, -8064563587743129892),
            (2, 0, 0),
            (2, 0, 66718116),
            (2, 0, 733950369757766878),
            (2, 0, 2044789983495278802),
            (2, 0, 2403967656666566197),
            (2, 4, -9199293364914546702),
            (2, 4, -9104424882442202562),
            (2, 4, -8942117006530427874),
            (2, 4, -8805351871358148900),
            (2, 4, -8237347600058197888),
        ];
        edges_sanity_check_inner(edges, 3, 5, 6)
    }

    #[test]
    fn edges_sanity_chunk_1() {
        edges_sanity_check_inner(vec![(876787706323152993, 0, 0)], 1, 1, 1)
    }

    #[test]
    fn edges_sanity_chunk_2() {
        edges_sanity_check_inner(vec![(4, 3, 2), (4, 5, 0)], 2, 2, 2)
    }

    #[test]
    fn large_failing_edge_sanity_repeated() {
        let edges = vec![
            (0, 0, 0),
            (0, 1, 0),
            (0, 2, 0),
            (0, 3, 0),
            (0, 4, 0),
            (0, 5, 0),
            (0, 6, -30),
            (4, 7, -83),
            (4, 7, -77),
            (6, 8, -68),
            (6, 8, -65),
            (9, 10, 46),
            (9, 10, 46),
            (9, 10, 51),
            (9, 10, 54),
            (9, 10, 59),
            (9, 10, 59),
            (9, 10, 59),
            (9, 10, 65),
            (9, 11, -75),
        ];
        let input_chunk_size = 411;
        let edge_chunk_size = 5;
        let edge_max_list_size = 7;

        edges_sanity_check_inner(edges, input_chunk_size, edge_chunk_size, edge_max_list_size);
    }

    #[test]
    fn edge_sanity_chunk_broken_incoming() {
        let edges = vec![
            (0, 0, 0),
            (0, 0, 0),
            (0, 0, 66),
            (0, 1, 0),
            (2, 0, 0),
            (3, 4, 0),
            (4, 0, 0),
            (4, 4, 0),
            (4, 4, 0),
            (4, 4, 0),
            (4, 4, 0),
            (5, 0, 0),
            (6, 7, 7274856480798084567),
            (8, 3, -7707029126214574305),
        ];

        edges_sanity_check_inner(edges, 853, 122, 98)
    }

    #[test]
    fn edge_sanity_chunk_broken_something() {
        let edges = vec![(0, 3, 0), (1, 2, 0), (3, 2, 0)];
        edges_sanity_check_inner(edges, 1, 1, 1)
    }

    #[test]
    fn test_reload() {
        let graph_dir = TempDir::new().unwrap();
        let graph = Graph::new();
        graph.add_edge(0, 0, 1, [("weight", 0.)], None).unwrap();
        graph.add_edge(1, 0, 1, [("weight", 1.)], None).unwrap();
        graph.add_edge(2, 0, 1, [("weight", 2.)], None).unwrap();
        graph.add_edge(3, 1, 2, [("weight", 3.)], None).unwrap();
        let disk_graph = graph.persist_as_disk_graph(graph_dir.path()).unwrap();
        let graph = disk_graph.inner.layer(0);

        let all_exploded: Vec<_> = graph
            .exploded_edges()
            .map(|e| (e.src(), e.dst(), e.timestamp()))
            .collect();
        let expected: Vec<_> = vec![
            (VID(0), VID(1), 0),
            (VID(0), VID(1), 1),
            (VID(0), VID(1), 2),
            (VID(1), VID(2), 3),
        ];
        assert_eq!(all_exploded, expected);

        let node_gids = PrimitiveArray::from_slice([0u64, 1, 2]).boxed();
        let reloaded_graph =
            TempColGraphFragment::new(graph.graph_dir(), true, 0, node_gids).unwrap();

        check_graph_sanity(
            &[(0, 1, 0), (0, 1, 1), (0, 1, 2), (1, 2, 3)],
            &[0, 1, 2],
            &reloaded_graph,
        );
    }

    mod addition_bounds {
        use itertools::Itertools;
        use proptest::{prelude::*, sample::size_range};
        use raphtory_api::core::entities::VID;
        use tempfile::TempDir;

        use super::{
            edges_sanity_check_build_graph, AdditionOps, Graph, GraphViewOps, NodeViewOps,
            TempColGraphFragment, NO_PROPS,
        };

        fn compare_raphtory_graph(edges: Vec<(u64, u64, i64)>, chunk_size: usize) {
            let nodes = edges
                .iter()
                .flat_map(|(src, dst, _)| [*src, *dst])
                .sorted()
                .dedup()
                .collect::<Vec<_>>();

            let rg = Graph::new();

            for (src, dst, time) in &edges {
                rg.add_edge(*time, *src, *dst, NO_PROPS, None)
                    .expect("failed to add edge");
            }

            let test_dir = TempDir::new().unwrap();
            let graph: TempColGraphFragment = edges_sanity_check_build_graph(
                test_dir.path(),
                &edges,
                &nodes,
                edges.len() as u64,
                chunk_size,
                chunk_size,
            )
            .unwrap();

            for (v_id, node) in nodes.into_iter().enumerate() {
                let node = rg.node(node).expect("failed to get node id");
                let expected = node.history();
                let node = graph.node(VID(v_id));
                let actual = node.timestamps().into_iter_t().collect::<Vec<_>>();
                assert_eq!(actual, expected);
            }
        }

        #[test]
        fn node_additions_bounds_to_arrays() {
            let edges = vec![(0, 0, -2), (0, 0, -1), (0, 0, 0), (0, 0, 1), (0, 0, 2)];

            compare_raphtory_graph(edges, 2);
        }

        #[test]
        fn test_load_from_graph_missing_edge() {
            let g = Graph::new();
            g.add_edge(0, 1, 2, [("test", "test1")], Some("1")).unwrap();
            g.add_edge(1, 2, 3, [("test", "test2")], Some("2")).unwrap();
            let test_dir = TempDir::new().unwrap();
            let _ = g.persist_as_disk_graph(test_dir.path()).unwrap();
        }

        #[test]
        fn one_edge_bounds_chunk_remainder() {
            let edges = vec![(0u64, 1, 0)];
            compare_raphtory_graph(edges, 3);
        }

        #[test]
        fn same_edge_twice() {
            let edges = vec![(0, 1, 0), (0, 1, 1)];
            compare_raphtory_graph(edges, 3);
        }

        proptest! {
            #[test]
            fn node_addition_bounds_test(
                edges in any_with::<Vec<(u8, u8, Vec<i64>)>>(size_range(1..=100).lift()).prop_map(|v| {
                    let mut v: Vec<(u64, u64, i64)> = v.into_iter().flat_map(|(src, dst, times)| {
                        let src = src as u64;
                        let dst = dst as u64;
                        times.into_iter().map(move |t| (src, dst, t))}).collect();
                    v.sort();
                    v}).prop_filter("edge list mut have one edge at least",|edges| !edges.is_empty()),
                chunk_size in 1..300usize,
            ) {
                compare_raphtory_graph(edges, chunk_size);
            }
        }
    }
}
