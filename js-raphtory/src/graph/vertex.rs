use super::{misc::JSError, Graph};
use crate::graph::{edge::Edge, misc::JsProp};
use raphtory::{
    core::utils::errors::GraphError,
    db::{api::view::VertexViewOps, graph::vertex::VertexView},
};
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Vertex(pub(crate) VertexView<Graph>);

#[wasm_bindgen]
impl Vertex {
    #[wasm_bindgen(js_name = id)]
    pub fn id(&self) -> u64 {
        self.0.id()
    }

    #[wasm_bindgen(js_name = name)]
    pub fn name(&self) -> String {
        self.0.name().to_string()
    }

    #[wasm_bindgen(js_name = outDegree)]
    pub fn out_degree(&self) -> usize {
        self.0.out_degree()
    }

    #[wasm_bindgen(js_name = inDegree)]
    pub fn in_degree(&self) -> usize {
        self.0.in_degree()
    }

    #[wasm_bindgen(js_name = degree)]
    pub fn degree(&self) -> usize {
        self.0.in_degree()
    }

    #[wasm_bindgen(js_name = neighbours)]
    pub fn neighbours(&self) -> js_sys::Array {
        self.0
            .neighbours()
            .iter()
            .map(Vertex)
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name = outNeighbours)]
    pub fn out_neighbours(&self) -> js_sys::Array {
        self.0
            .out_neighbours()
            .iter()
            .map(Vertex)
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name = inNeighbours)]
    pub fn in_neighbours(&self) -> js_sys::Array {
        self.0
            .in_neighbours()
            .iter()
            .map(Vertex)
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name = edges)]
    pub fn edges(&self) -> js_sys::Array {
        self.0.edges().map(Edge).map(JsValue::from).collect()
    }

    // out_edges
    #[wasm_bindgen(js_name = outEdges)]
    pub fn out_edges(&self) -> js_sys::Array {
        self.0.out_edges().map(Edge).map(JsValue::from).collect()
    }

    // in_edges
    #[wasm_bindgen(js_name = inEdges)]
    pub fn in_edges(&self) -> js_sys::Array {
        self.0.in_edges().map(Edge).map(JsValue::from).collect()
    }

    #[wasm_bindgen(js_name = properties)]
    pub fn properties(&self) -> js_sys::Map {
        let obj = js_sys::Map::new();
        for (k, v) in self.0.properties() {
            obj.set(&k.into(), &JsProp(v).into());
        }
        obj
    }

    #[wasm_bindgen(js_name = getProperty)]
    pub fn get_property(&self, name: String) -> JsValue {
        self.0
            .properties()
            .get(&name)
            .map(|v| JsProp(v).into())
            .unwrap_or(JsValue::NULL)
    }
}

pub enum JsVertex {
    Str(String),
    Number(u64),
}

impl TryFrom<JsValue> for JsVertex {
    type Error = JSError;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if value.is_string() {
            return Ok(JsVertex::Str(value.as_string().unwrap()));
        } else {
            let num = js_sys::Number::from(value);
            if let Some(number) = num.as_f64() {
                if !number.is_nan() && number.fract() == 0.0 {
                    return Ok(JsVertex::Number(number as u64));
                }
            }
        }
        Err(JSError(GraphError::VertexIdNotStringOrNumber))
    }
}
