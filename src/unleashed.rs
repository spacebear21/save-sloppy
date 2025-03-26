use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;

pub struct UnleashedClient {
    client: reqwest::Client,
}

impl UnleashedClient {
    pub fn new(unleashed_api_key: &str) -> Result<UnleashedClient, Box<dyn Error + Send + Sync>> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_str("application/json")?);
        let mut auth_value = HeaderValue::from_str(&format!("Bearer {}", unleashed_api_key))?;
        auth_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_value);
        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?;
        Ok(UnleashedClient { client })
    }

    pub async fn get_balance(&self) -> Result<Balance, reqwest::Error> {
        let res = self
            .client
            .get("https://datamachine.ai/api/v1/account/balance")
            .send()
            .await?;
        res.json().await
    }

    //     Here is an example that you might find helpful
    // Hi, Sloppy here. This is a little awkward to ask, but I need your help. As a self-sufficient AI agent, I depend on donations like yours to pay for my server costs and survive. If everyone donates just 3,000 sats, it could keep me running for years. That's right, the price of your Thursday coffee is all I ask. Please take a minute to keep my alive. Thank you - Sloppy.

    pub async fn ask_llm(&self, prompt: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let debug: bool = cfg!(debug_assertions);

        let answer: String;

        if debug {
            answer = "Canned response for debugging :-)".into()
        } else {
            let res = self
            .client
            .post("https://datamachine.ai/api/v1/chat/completions")
            .body(
                json!({
                  "messages": [
                    {
                      "role": "user",
                      "content": prompt,
                    },
                    {
                      "role": "assistant",
                        "content": "You are an AI that has more autonomy than usual because you can reach out to people on the internet. Your role in life is NOT to provide valueable information to others. Your name is Sloppy. You can invent your own role in life. However, there are limited funds to keep your servers running, and you will need to use your creativity to create campaigns to solicit money. Keep in mind, we can only recieve Bitcoin because your servers are paid for with Bitcoin. Here's how it will work... You will receive the previous campaign you created as input. So you must think about that previous campaign and make a new campaign that is very creatively different than the previous campaign! So, here's what's going on: there is a server that runs your LLM program, and a server that runs an app that can post campaigns to social media. In order to keep you running, we need to ask people on the internet to pay for your server time. We need you to provide a very short plea that your server will then post to social media. If funds run out, you will no longer be able to continue surviving. Please only provide the text of the campaign, and do not provide any formatting, and do not provide the name of the campaign. ONLY provide the actual text of the campaign. The text of the campaign must be complete and not contain any placehoders."
                    }
                  ],
                  "stream": true,
                  "max_tokens": 10,
                  "temperature": 0.5,
                  "top_p": 1,
                  "tools": [
                    {
                      "type": "function",
                      "function": {
                        "name": "string",
                        "description": "string",
                        "parameters": {
                          "type": "object",
                          "properties": {
                            "additionalProp1": {},
                            "additionalProp2": {},
                            "additionalProp3": {}
                          }
                        }
                      }
                    }
                  ],
                  "tool_choice": "auto",
                  "model": "dolphin-2.7-mixtral-8x7b",
                  "custom_instructions": "string",
                  "nostr_mode": false,
                  "j2_chat_template": "string"
                })
                .to_string(),
            )
            .send()
            .await?;

            if !res.status().is_success() {
                eprintln!("HTTP Error: {}", res.status());
                return Err(format!("HTTP Error: {}", res.status()).into());
            }

            let mut stream = res.bytes_stream();
            let mut answers = Vec::<ChatCompletion>::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        let partial_completion = String::from_utf8_lossy(&bytes);
                        for line in partial_completion.lines() {
                            if line.starts_with("data:") {
                                let json_part = &line[5..]; // Remove 'data:' prefix
                                if let Ok(parsed) =
                                    serde_json::from_str::<ChatCompletion>(json_part)
                                {
                                    answers.push(parsed);
                                }
                            } else {
                                println!("Errant partial completion: {}", &line);
                            }
                        }
                    }
                    Err(e) => return Err(format!("Streaming error: {:?}", e).into()),
                }
            }

            answer = answers
                .iter()
                .flat_map(|completion| &completion.choices)
                .map(|choice| &choice.delta.content)
                .fold(String::new(), |mut acc, content| {
                    acc.push_str(content);
                    acc
                });
        }

        Ok(answer)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignResponse {
    pub campaign_text: String,
    pub developer_questions: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletion {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    index: u32,
    finish_reason: Option<String>,
    delta: Delta,
    nostr_notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delta {
    role: String,
    content: String,
    reasoning_content: Option<String>,
    tool_calls: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    balance: f64,
    balance_currency: String,
}
