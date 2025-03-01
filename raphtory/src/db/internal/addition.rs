use crate::{
    core::{
        entities::{graph::tgraph::InternalGraph, EID, VID},
        storage::timeindex::TimeIndexEntry,
        utils::errors::GraphError,
        PropType,
    },
    db::api::mutation::internal::InternalAdditionOps,
    prelude::Prop,
};
use std::sync::atomic::Ordering;

impl InternalAdditionOps for InternalGraph {
    #[inline]
    fn next_event_id(&self) -> usize {
        self.inner().event_counter.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    fn resolve_layer(&self, layer: Option<&str>) -> usize {
        layer
            .map(|name| self.inner().edge_meta.get_or_create_layer_id(name))
            .unwrap_or(0)
    }

    #[inline]
    fn resolve_node_type(&self, v_id: VID, node_type: Option<&str>) -> Result<usize, GraphError> {
        self.inner().resolve_node_type(v_id, node_type)
    }

    #[inline]
    fn resolve_node(&self, id: u64, name: Option<&str>) -> VID {
        self.inner().resolve_node(id, name)
    }

    #[inline]
    fn resolve_graph_property(&self, prop: &str, is_static: bool) -> usize {
        self.inner().graph_meta.resolve_property(prop, is_static)
    }

    #[inline]
    fn resolve_node_property(
        &self,
        prop: &str,
        dtype: PropType,
        is_static: bool,
    ) -> Result<usize, GraphError> {
        self.inner()
            .node_meta
            .resolve_prop_id(prop, dtype, is_static)
    }

    #[inline]
    fn resolve_edge_property(
        &self,
        prop: &str,
        dtype: PropType,
        is_static: bool,
    ) -> Result<usize, GraphError> {
        self.inner()
            .edge_meta
            .resolve_prop_id(prop, dtype, is_static)
    }

    #[inline]
    fn process_prop_value(&self, prop: Prop) -> Prop {
        match prop {
            Prop::Str(value) => Prop::Str(self.inner().resolve_str(value)),
            _ => prop,
        }
    }

    #[inline]
    fn internal_add_node(
        &self,
        t: TimeIndexEntry,
        v: VID,
        props: Vec<(usize, Prop)>,
        node_type_id: usize,
    ) -> Result<(), GraphError> {
        self.inner().add_node_internal(t, v, props, node_type_id)
    }

    #[inline]
    fn internal_add_edge(
        &self,
        t: TimeIndexEntry,
        src: VID,
        dst: VID,
        props: Vec<(usize, Prop)>,
        layer: usize,
    ) -> Result<EID, GraphError> {
        self.inner().add_edge_internal(t, src, dst, props, layer)
    }
}
