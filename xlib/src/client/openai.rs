use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    base_url: String,
}

pub struct OpenAIClientConfig {
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub input: String,
    pub model: String,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
}

impl OpenAIClient {
    pub fn new(config: OpenAIClientConfig) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .context("Invalid API key format")?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            api_key: config.api_key,
            base_url: config.base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        })
    }

    pub async fn create_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            input: text.to_string(),
            model: "text-embedding-3-small".to_string(),
        };

        let response = self
            .client
            .post(&format!("{}/embeddings", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send embedding request")?;

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse embedding response")?;

        embedding_response
            .data
            .into_iter()
            .next()
            .map(|data| data.embedding)
            .context("No embedding data received")
    }

    pub async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        json_mode: bool,
    ) -> Result<String> {
        let mut request = ChatRequest {
            model: "gpt-4o".to_string(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2000),
            response_format: None,
        };

        if json_mode {
            request.response_format = Some(ResponseFormat {
                format_type: "json_object".to_string(),
            });
        }

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat completion request")?;

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse chat completion response")?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .context("No chat completion received")
    }
} 