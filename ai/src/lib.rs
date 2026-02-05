use std::future::Future;
use std::{fmt::Display, sync::Arc};

use mistralrs::{Model, Response, TextMessageRole, TextMessages, TextModelBuilder};

pub struct LLM {
    model: Model,
    history: Vec<(TextMessageRole, String)>,
}

impl LLM {
    pub async fn new() -> Self {
        let model = TextModelBuilder::new("Qwen/Qwen3-0.6B")
            .with_dtype(mistralrs::ModelDType::F16)
            .with_tool_callback(
                "execute_query",
                Arc::new(|params| Ok(format!("{}({})", params.name, params.arguments))),
            )
            .build()
            .await
            .expect("Couldn't get model");

        LLM {
            model,
            history: vec![],
        }
    }

    pub async fn set_system_prompt(&mut self, prompt: impl Display) {
        self.history
            .push((TextMessageRole::System, prompt.to_string()));
    }

    pub async fn get_completion(&mut self, prompt: impl Display) -> Result<String, String> {
        self.history
            .push((TextMessageRole::User, prompt.to_string()));

        let messages = self
            .history
            .iter()
            .fold(TextMessages::new(), |msgs, (role, content)| {
                msgs.add_message(role.clone(), content.clone())
            });

        let response = self
            .model
            .send_chat_request(messages)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(answer) = response.choices[0].message.content.as_ref() {
            self.history
                .push((TextMessageRole::Assistant, answer.clone()));
            return Ok(answer.clone());
        }

        Err("No answer".to_string())
    }

    pub async fn stream_completion<F, Fut>(
        &mut self,
        prompt: impl Display,
        mut on_chunk: F,
    ) -> Result<String, String>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = ()>,
    {
        self.history
            .push((TextMessageRole::User, prompt.to_string()));

        let messages = self
            .history
            .iter()
            .fold(TextMessages::new(), |msgs, (role, content)| {
                msgs.add_message(role.clone(), content.clone())
            });

        let mut stream = self
            .model
            .stream_chat_request(messages)
            .await
            .map_err(|e| e.to_string())?;

        let mut full_response = String::new();

        while let Some(chunk) = stream.next().await {
            if let Response::Chunk(chunk_response) = chunk {
                if let Some(choice) = chunk_response.choices.first()
                    && let Some(content) = &choice.delta.content
                {
                    full_response.push_str(content);
                    on_chunk(content.clone()).await;
                }
                if let Some(choice) = chunk_response.choices.first()
                    && let Some(tool) = &choice.delta.tool_calls
                    && let Some(call) = tool.first()
                {
                    on_chunk(format!(
                        "{}({})",
                        call.function.name, call.function.arguments
                    ));
                }
            }
        }

        self.history
            .push((TextMessageRole::Assistant, full_response.clone()));

        Ok(full_response)
    }
}
