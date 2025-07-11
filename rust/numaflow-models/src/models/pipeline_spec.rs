/*
Copyright 2022 The Numaproj Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

// Code generated by Openapi Generator. DO NOT EDIT.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Edges define the relationships between vertices
    #[serde(rename = "edges", skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<crate::models::Edge>>,
    #[serde(rename = "interStepBuffer", skip_serializing_if = "Option::is_none")]
    pub inter_step_buffer: Option<Box<crate::models::InterStepBuffer>>,
    /// InterStepBufferServiceName is the name of the InterStepBufferService to be used by the pipeline
    #[serde(
        rename = "interStepBufferServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_step_buffer_service_name: Option<String>,
    #[serde(rename = "lifecycle", skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<Box<crate::models::Lifecycle>>,
    #[serde(rename = "limits", skip_serializing_if = "Option::is_none")]
    pub limits: Option<Box<crate::models::PipelineLimits>>,
    /// SideInputs defines the Side Inputs of a pipeline.
    #[serde(rename = "sideInputs", skip_serializing_if = "Option::is_none")]
    pub side_inputs: Option<Vec<crate::models::SideInput>>,
    #[serde(rename = "templates", skip_serializing_if = "Option::is_none")]
    pub templates: Option<Box<crate::models::Templates>>,
    #[serde(rename = "vertices", skip_serializing_if = "Option::is_none")]
    pub vertices: Option<Vec<crate::models::AbstractVertex>>,
    #[serde(rename = "watermark", skip_serializing_if = "Option::is_none")]
    pub watermark: Option<Box<crate::models::Watermark>>,
}

impl PipelineSpec {
    pub fn new() -> PipelineSpec {
        PipelineSpec {
            edges: None,
            inter_step_buffer: None,
            inter_step_buffer_service_name: None,
            lifecycle: None,
            limits: None,
            side_inputs: None,
            templates: None,
            vertices: None,
            watermark: None,
        }
    }
}
