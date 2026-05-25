// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod fs;
pub mod git;
mod multi;

use genai::chat::{Tool, ToolName};
pub use multi::*;
use schemars::JsonSchema;
use serde_json::{Map, Value};

pub fn tool_description<T: JsonSchema>(name: &'static str, description: &'static str) -> Tool {
    Tool {
        name: ToolName::Custom(name.to_string()),
        description: Some(description.to_string()),
        schema: Some({
            let mut settings = schemars::generate::SchemaSettings::openapi3();
            settings.inline_subschemas = true;
            let generator = schemars::generate::SchemaGenerator::new(settings);
            normalize_tool_schema(generator.into_root_schema_for::<T>().to_value())
        }),
        strict: Some(true),
        config: None,
    }
}

fn normalize_tool_schema(mut schema: Value) -> Value {
    let Some(object) = schema.as_object_mut() else {
        return schema;
    };

    if object.get("type").and_then(Value::as_str) == Some("object") {
        object
            .entry("properties".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        object
            .entry("required".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
    }

    schema
}

pub trait MarkerAgentTool {
    fn tool(&self) -> Tool;
}

#[async_trait::async_trait]
pub trait AgentTool: Send + Sync {
    fn tool(&self) -> Tool;

    async fn run(&self, args: &serde_json::Value) -> anyhow::Result<String>;
}

#[cfg(test)]
mod tests {
    use super::tool_description;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, JsonSchema)]
    struct NoArgs {}

    #[derive(Debug, Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct ReadFileArgs {
        path: String,
    }

    #[test]
    fn adds_empty_properties_for_no_arg_tools() {
        let tool = tool_description::<NoArgs>("no_args", "No arguments");
        let schema = tool.schema.expect("schema should be present");

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], serde_json::json!({}));
        assert_eq!(schema["required"], serde_json::json!([]));
    }

    #[test]
    fn preserves_properties_for_tools_with_arguments() {
        let tool = tool_description::<ReadFileArgs>("read_file", "Read a file");
        let schema = tool.schema.expect("schema should be present");

        assert_eq!(schema["properties"]["path"]["type"], "string");
        assert_eq!(schema["required"], serde_json::json!(["path"]));
    }
}
