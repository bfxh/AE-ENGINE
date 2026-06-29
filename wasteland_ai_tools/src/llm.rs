use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub model_path: String,
    pub context_size: usize,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub repeat_penalty: f32,
    pub threads: u32,
    pub gpu_layers: u32,
    pub batch_size: usize,
    pub quantized: bool,
    pub quantization_bits: u8,
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig {
            model_path: String::new(),
            context_size: 4096,
            max_tokens: 512,
            temperature: 0.7,
            top_p: 0.9,
            repeat_penalty: 1.1,
            threads: 4,
            gpu_layers: 0,
            batch_size: 512,
            quantized: true,
            quantization_bits: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Npc,
    World,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tokens_used: u32,
    pub tokens_per_second: f32,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    Error,
}

pub struct LlmContext {
    messages: Vec<ChatMessage>,
    max_messages: usize,
    summary: Option<String>,
    memory: ContextMemory,
}

#[derive(Debug, Clone, Default)]
pub struct ContextMemory {
    key_facts: Vec<String>,
    relationship_scores: HashMap<String, f32>,
    world_state: HashMap<String, String>,
    recent_events: Vec<String>,
}

impl LlmContext {
    pub fn new(max_messages: usize) -> Self {
        LlmContext {
            messages: Vec::with_capacity(max_messages),
            max_messages,
            summary: None,
            memory: ContextMemory::default(),
        }
    }

    pub fn add_message(&mut self, msg: ChatMessage) {
        if self.messages.len() >= self.max_messages {
            self.compress_context();
        }
        self.messages.push(msg);
    }

    pub fn set_system_prompt(&mut self, prompt: &str) {
        if self.messages.first().is_none_or(|m| m.role != MessageRole::System) {
            self.messages.insert(
                0,
                ChatMessage {
                    role: MessageRole::System,
                    content: prompt.to_string(),
                    tool_calls: None,
                },
            );
        } else {
            self.messages[0].content = prompt.to_string();
        }
    }

    fn compress_context(&mut self) {
        if self.messages.len() <= 2 {
            return;
        }
        let sys_msg = if self.messages[0].role == MessageRole::System {
            self.messages.remove(0)
        } else {
            ChatMessage { role: MessageRole::System, content: String::new(), tool_calls: None }
        };
        let old_messages: Vec<String> = self
            .messages
            .iter()
            .map(|m| format!("[{}]: {}", format!("{:?}", m.role).to_lowercase(), m.content))
            .collect();
        self.summary = Some(old_messages.join("\n"));
        self.messages.clear();
        self.messages.push(sys_msg);
        if let Some(ref summary) = self.summary {
            self.messages.push(ChatMessage {
                role: MessageRole::System,
                content: format!("Previous conversation summary:\n{}", summary),
                tool_calls: None,
            });
        }
    }

    pub fn context_window_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.content.len() / 4).sum()
    }

    pub fn add_fact(&mut self, fact: String) {
        if !self.memory.key_facts.contains(&fact) {
            self.memory.key_facts.push(fact);
        }
    }

    pub fn set_relationship(&mut self, npc_id: &str, score: f32) {
        self.memory.relationship_scores.insert(npc_id.to_string(), score);
    }

    pub fn update_world_state(&mut self, key: &str, value: &str) {
        self.memory.world_state.insert(key.to_string(), value.to_string());
    }

    pub fn add_event(&mut self, event: String) {
        self.memory.recent_events.push(event);
        if self.memory.recent_events.len() > 50 {
            self.memory.recent_events.remove(0);
        }
    }

    pub fn build_context_string(&self) -> String {
        let mut ctx = String::new();
        if !self.memory.key_facts.is_empty() {
            ctx.push_str("Key facts:\n");
            for fact in &self.memory.key_facts {
                ctx.push_str(&format!("- {}\n", fact));
            }
        }
        if !self.memory.recent_events.is_empty() {
            ctx.push_str("Recent events:\n");
            for event in self.memory.recent_events.iter().rev().take(5) {
                ctx.push_str(&format!("- {}\n", event));
            }
        }
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_message() {
        let mut ctx = LlmContext::new(10);
        ctx.add_message(ChatMessage {
            role: MessageRole::User,
            content: "hello".into(),
            tool_calls: None,
        });
        assert_eq!(ctx.messages.len(), 1);
    }

    #[test]
    fn test_system_prompt() {
        let mut ctx = LlmContext::new(10);
        ctx.set_system_prompt("You are a wasteland NPC.");
        assert_eq!(ctx.messages[0].role, MessageRole::System);
        ctx.set_system_prompt("New prompt.");
        assert_eq!(ctx.messages[0].content, "New prompt.");
    }

    #[test]
    fn test_context_compression() {
        let mut ctx = LlmContext::new(3);
        ctx.set_system_prompt("system");
        ctx.add_message(ChatMessage {
            role: MessageRole::User,
            content: "msg1".into(),
            tool_calls: None,
        });
        ctx.add_message(ChatMessage {
            role: MessageRole::Assistant,
            content: "msg2".into(),
            tool_calls: None,
        });
        ctx.add_message(ChatMessage {
            role: MessageRole::User,
            content: "msg3".into(),
            tool_calls: None,
        });
        assert!(ctx.summary.is_some());
        assert!(ctx.messages.len() <= 3);
    }

    #[test]
    fn test_memory_facts() {
        let mut ctx = LlmContext::new(10);
        ctx.add_fact("Player found a key".into());
        ctx.add_fact("Player found a key".into());
        assert_eq!(ctx.memory.key_facts.len(), 1);
    }

    #[test]
    fn test_world_state() {
        let mut ctx = LlmContext::new(10);
        ctx.update_world_state("time", "night");
        ctx.update_world_state("weather", "storm");
        assert_eq!(ctx.memory.world_state.get("time").unwrap(), "night");
        assert_eq!(ctx.memory.world_state.get("weather").unwrap(), "storm");
    }
}
