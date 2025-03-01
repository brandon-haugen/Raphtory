use crate::graph::{Graph, UnderGraph};
use chrono::{Datelike, Timelike};
use js_sys::Array;
use raphtory::core::{utils::errors::GraphError, DocumentInput, Prop};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
#[derive(Debug)]
pub struct JSError(pub(crate) GraphError);

pub(crate) struct JsObjectEntry(pub(crate) JsValue);

#[derive(Serialize, Deserialize)]
#[repr(transparent)]
pub(crate) struct JsProp(pub(crate) Prop);

#[wasm_bindgen]
#[derive(Debug)]
pub struct JSDocumentProp(DocumentInput);

impl From<JsProp> for JsValue {
    fn from(value: JsProp) -> JsValue {
        match value.0 {
            Prop::U8(v) => v.into(),
            Prop::U16(v) => v.into(),
            Prop::Str(v) => v.to_string().into(),
            Prop::I32(v) => v.into(),
            Prop::I64(v) => v.into(),
            Prop::U32(v) => v.into(),
            Prop::U64(v) => v.into(),
            Prop::F32(v) => v.into(),
            Prop::F64(v) => v.into(),
            Prop::Bool(v) => v.into(),
            Prop::DTime(v) => js_sys::Date::new_with_year_month_day_hr_min_sec_milli(
                v.year() as u32,
                v.month() as i32,
                v.day() as i32,
                v.hour() as i32,
                v.minute() as i32,
                v.second() as i32,
                0,
            )
            .into(),
            Prop::NDTime(v) => js_sys::Date::new_with_year_month_day_hr_min_sec_milli(
                v.year() as u32,
                v.month() as i32,
                v.day() as i32,
                v.hour() as i32,
                v.minute() as i32,
                v.second() as i32,
                0,
            )
            .into(),
            Prop::Graph(v) => Graph(UnderGraph::TGraph(Arc::new(v))).into(),
            Prop::PersistentGraph(v) => todo!("PersistentGraph not yet implemented"),
            Prop::List(v) => {
                let v: Array = v.iter().map(|v| JsValue::from(JsProp(v.clone()))).collect();
                v.into()
            }
            Prop::Map(v) => {
                let v = v.deref().clone();
                serde_wasm_bindgen::to_value(&v).unwrap()
            }
            Prop::Document(doc) => JSDocumentProp(doc).into(),
        }
    }
}

impl Deref for JsProp {
    type Target = Prop;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<JsObjectEntry> for Option<(String, Prop)> {
    fn from(entry: JsObjectEntry) -> Self {
        let arr: Array = entry.0.into();

        let key = arr.at(0).as_string().unwrap();
        let value = arr.at(1).as_string().unwrap();
        Some((key, Prop::str(value)))
    }
}
