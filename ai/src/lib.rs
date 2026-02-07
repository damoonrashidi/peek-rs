use std::fmt::Display;
use std::future::Future;

use mistralrs::{Model, RequestBuilder, Response, TextMessageRole, TextModelBuilder, ToolChoice};

// Re-export types that consumers will need to create and use tools
pub use mistralrs::{Function, Tool, ToolType};
pub use serde_json::{Value, json};
pub use std::collections::HashMap;

/// Information about a tool call from the model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Represents a chunk in the streaming response
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Regular text content
    Text(String),
    /// A tool call request from the model
    ToolCall(ToolCallInfo),
}

pub struct LLM {
    model: Model,
    history: Vec<(TextMessageRole, String)>,
    tools: Vec<Tool>,
}

impl LLM {
    pub async fn new() -> Self {
        let conf = config::PeekConfig::get_or_default();
        let model = TextModelBuilder::new(conf.ai.model)
            .with_dtype(mistralrs::ModelDType::F16)
            .build()
            .await
            .expect("Couldn't get model");

        LLM {
            model,
            history: vec![],
            tools: vec![],
        }
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }

    /// Set all tools for the LLM, replacing any existing tools
    pub fn set_tools(&mut self, tools: Vec<Tool>) {
        self.tools = tools;
    }

    pub async fn set_system_prompt(&mut self, prompt: impl Display) {
        self.history
            .push((TextMessageRole::System, prompt.to_string()));
    }

    pub async fn stream_completion<F, Fut>(
        &mut self,
        prompt: impl Display,
        mut on_chunk: F,
    ) -> Result<Vec<ToolCallInfo>, String>
    where
        F: FnMut(StreamChunk) -> Fut,
        Fut: Future<Output = ()>,
    {
        self.history
            .push((TextMessageRole::User, prompt.to_string()));

        let mut request_builder = self
            .history
            .iter()
            .fold(RequestBuilder::new(), |builder, (role, content)| {
                builder.add_message(role.clone(), content.clone())
            });

        if !self.tools.is_empty() {
            request_builder = request_builder
                .set_tools(self.tools.clone())
                .set_tool_choice(ToolChoice::Auto);
        }

        let request_builder = request_builder;

        let mut stream = self
            .model
            .stream_chat_request(request_builder)
            .await
            .map_err(|e| e.to_string())?;

        let mut full_response = String::new();
        let mut tool_calls: Vec<ToolCallInfo> = vec![];

        while let Some(chunk) = stream.next().await {
            if let Response::Chunk(chunk_response) = chunk {
                if let Some(choice) = chunk_response.choices.first()
                    && let Some(content) = &choice.delta.content
                {
                    full_response.push_str(content);
                    on_chunk(StreamChunk::Text(content.clone())).await;
                }
                if let Some(choice) = chunk_response.choices.first()
                    && let Some(tool) = &choice.delta.tool_calls
                    && let Some(call) = tool.first()
                {
                    let tool_call_info = ToolCallInfo {
                        id: call.id.clone(),
                        name: call.function.name.clone(),
                        arguments: call.function.arguments.clone(),
                    };
                    tool_calls.push(tool_call_info.clone());
                    on_chunk(StreamChunk::ToolCall(tool_call_info)).await;
                }
            }
        }

        self.history
            .push((TextMessageRole::Assistant, full_response.clone()));

        Ok(tool_calls)
    }

    /// Add a tool result to the conversation history and continue
    pub async fn add_tool_result<F, Fut>(
        &mut self,
        tool_call_id: String,
        result: String,
        mut on_chunk: F,
    ) -> Result<Vec<ToolCallInfo>, String>
    where
        F: FnMut(StreamChunk) -> Fut,
        Fut: Future<Output = ()>,
    {
        // Add tool result to history
        self.history.push((
            TextMessageRole::Tool,
            serde_json::json!({
                "tool_call_id": tool_call_id,
                "content": result,
            })
            .to_string(),
        ));

        // Build request with updated history
        let mut request_builder = self
            .history
            .iter()
            .fold(RequestBuilder::new(), |builder, (role, content)| {
                builder.add_message(role.clone(), content.clone())
            });

        if !self.tools.is_empty() {
            request_builder = request_builder
                .set_tools(self.tools.clone())
                .set_tool_choice(ToolChoice::Auto);
        }

        let mut stream = self
            .model
            .stream_chat_request(request_builder)
            .await
            .map_err(|e| e.to_string())?;

        let mut full_response = String::new();
        let mut tool_calls: Vec<ToolCallInfo> = vec![];

        while let Some(chunk) = stream.next().await {
            if let Response::Chunk(chunk_response) = chunk {
                if let Some(choice) = chunk_response.choices.first()
                    && let Some(content) = &choice.delta.content
                {
                    full_response.push_str(content);
                    on_chunk(StreamChunk::Text(content.clone())).await;
                }
                if let Some(choice) = chunk_response.choices.first()
                    && let Some(tool) = &choice.delta.tool_calls
                    && let Some(call) = tool.first()
                {
                    let tool_call_info = ToolCallInfo {
                        id: call.id.clone(),
                        name: call.function.name.clone(),
                        arguments: call.function.arguments.clone(),
                    };
                    tool_calls.push(tool_call_info.clone());
                    on_chunk(StreamChunk::ToolCall(tool_call_info)).await;
                }
            }
        }

        self.history
            .push((TextMessageRole::Assistant, full_response.clone()));

        Ok(tool_calls)
    }

    /// Get the tools that are configured for this LLM
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }
}

/// Helper function to create a tool with the given name, description, and parameters
///
/// # Example
/// ```rust
/// use ai::{create_tool, json, HashMap, Value, ToolType};
///
/// let parameters: HashMap<String, Value> = serde_json::from_value(json!({
///     "type": "object",
///     "properties": {
///         "query": {
///             "type": "string",
///             "description": "The SQL query to execute",
///         },
///     },
///     "required": ["query"],
/// })).unwrap();
///
/// let tool = create_tool(
///     "query_database",
///     "Execute a SQL query against the database",
///     parameters,
/// );
/// ```
pub fn create_tool(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: HashMap<String, Value>,
) -> Tool {
    Tool {
        tp: ToolType::Function,
        function: Function {
            name: name.into(),
            description: Some(description.into()),
            parameters: Some(parameters),
        },
    }
}
