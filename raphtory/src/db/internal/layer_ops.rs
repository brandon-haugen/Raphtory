use crate::{
    core::{
        entities::{graph::tgraph::InternalGraph, LayerIds},
        utils::errors::GraphError,
    },
    db::api::view::internal::InternalLayerOps,
    prelude::Layer,
};

impl InternalLayerOps for InternalGraph {
    fn layer_ids(&self) -> &LayerIds {
        &LayerIds::All
    }

    fn layer_ids_from_names(&self, key: Layer) -> Result<LayerIds, GraphError> {
        self.inner().layer_ids(key)
    }

    fn valid_layer_ids_from_names(&self, key: Layer) -> LayerIds {
        self.inner().valid_layer_ids(key)
    }
}
