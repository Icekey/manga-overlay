use anyhow::{Result, anyhow, bail};
use reqwest::Client;
use serde_json::json;

use crate::OcrApp;
use crate::ui::shutdown::TASK_TRACKER;
use crate::ui::update_queue::enqueue_update;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    #[serde(default = "default_model_url")]
    pub model_url: String,

    pub model_names: Vec<String>,
    pub model_index: Option<usize>,

    #[serde(default = "default_prompt_template")]
    pub prompt_template: String,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            model_url: default_model_url(),
            model_names: vec![],
            model_index: None,
            prompt_template: default_prompt_template(),
        }
    }
}

fn default_model_url() -> String {
    "http://127.0.0.1:1337/v1".to_string()
}

//Prompt for lmg-anon/vntl-llama3-8b-v2-hf-q8_0
fn default_prompt_template() -> String {
    "<|begin_of_text|><|start_header_id|>system<|end_header_id|>

You are an expert translator. Translate the following Japanese text into natural, fluent English. \
Maintain the original tone and context.<|eot_id|><|start_header_id|>user<|end_header_id|>

{}<|eot_id|><|start_header_id|>assistant<|end_header_id|>"
        .to_string()
}

pub async fn translate(config: &TranslationConfig, jpn_text: &str) -> Result<String> {
    if config.model_index.is_none() {
        bail!("Model index not set");
    }
    let model_name = &config.model_names[config.model_index.unwrap()];

    if !config.prompt_template.contains("{}") {
        bail!("the placeholder '{{}}' is missing in the prompt template");
    }

    let payload = json!( {
        "model": model_name,
        "messages": [
            {
                "role": "user",
                "content": format!("{}", config.prompt_template.replace("{}", jpn_text))
            }
        ],
        "stream": false
    });

    let response = reqwest::Client::new()
        .post(format!("{}/chat/completions", &config.model_url))
        .json(&payload)
        .send()
        .await?;

    let body = response.json::<serde_json::Value>().await?;

    let result = body["choices"][0]["message"]["content"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Failed to get translation from local LLM"))?;

    Ok(result.to_string())
}

pub fn update_available_models(config: &TranslationConfig) {
    let config = config.clone();
    TASK_TRACKER.spawn(async move {
        let model_names = list_available_models(&config.model_url)
            .await
            .unwrap_or_default();

        enqueue_update(move |_, state: &mut OcrApp| {
            let config = &mut state.settings.translation_config;
            config.model_names = model_names;

            if config.model_names.is_empty() {
                config.model_index = None;
            } else if config.model_index.is_none() {
                config.model_index = Some(0);
            }
        });
    });
}

pub async fn list_available_models(model_url: &str) -> Result<Vec<String>> {
    let client = Client::new();

    let response = client.get(format!("{}/models", model_url)).send().await?;

    let body: serde_json::Value = response.json().await?;

    // Extract model names from the response
    let models = if let Some(data) = body.get("data") {
        let models_vec: Vec<String> = data
            .as_array()
            .ok_or_else(|| anyhow!("Expected data array in response"))?
            .iter()
            .map(|m| m.get("id").and_then(|n| n.as_str()).unwrap_or("Unknown"))
            .map(|name| name.to_string())
            .collect();

        models_vec
    } else {
        bail!("Expected data array in response")
    };

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_available_models() {
        let config = TranslationConfig::default();
        let models = list_available_models(config.model_url.as_str())
            .await
            .unwrap();
        println!("{:#?}", models);
    }

    #[tokio::test]
    async fn test_request_local_llm() {
        let config = TranslationConfig::default();
        let body = translate(
            &config,
            "今 いま 私 わたし\n は 東京 とうきょう に 住 す んでいるので",
        )
        .await
        .unwrap();
        println!("{}", body);
    }
}
