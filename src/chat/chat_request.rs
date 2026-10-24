use std::borrow::Cow;

use crate::chat::{ChatMessage, ChatRole, StreamEnd, Tool, ToolCall, ToolResponse};
use crate::support;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

// region:    --- ChatRequest

type Messages<'messages> = Cow<'messages, [ChatMessage]>;
type Tools<'tools> = Cow<'tools, [Tool]>;

/// A provider-neutral chat request containing conversation messages and available tools.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest<'messages, 'tools> {
	/// The initial system content of the request.
	pub system: Option<String>,

	/// The messages of the request.
	#[serde(default, borrow)]
	pub messages: Messages<'messages>,

	/// Optional tool definitions available to the model.
	pub tools: Option<Tools<'tools>>,

	/// Previous response ID used to continue a provider-managed stateful session.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub previous_response_id: Option<String>,

	/// Whether the provider should store the response for stateful continuation.
	///
	/// An unset value does not opt in to storage.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub store: Option<bool>,
}

/// Constructors
impl<'messages, 'tools> ChatRequest<'messages, 'tools> {
	/// Construct from a set of messages.
	pub fn new(messages: impl Into<Messages<'messages>>) -> Self {
		Self {
			messages: messages.into(),
			system: None,
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}

	/// Construct with an initial system prompt.
	pub fn from_system(content: impl Into<String>) -> Self {
		Self {
			system: Some(content.into()),
			messages: Default::default(),
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}

	/// Construct with a single user message.
	pub fn from_user(content: impl Into<String>) -> Self {
		Self {
			system: None,
			messages: vec![ChatMessage::user(content.into())].into(),
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}

	/// Construct from messages.
	pub fn from_messages(messages: impl Into<Messages<'messages>>) -> Self {
		Self {
			system: None,
			messages: messages.into(),
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}
}

/// Chainable Setters
impl<'messages, 'tools> ChatRequest<'messages, 'tools> {
	/// Set or replace the system prompt.
	pub fn with_system(mut self, system: impl Into<String>) -> Self {
		self.system = Some(system.into());
		self
	}

	/// Append one message.
	pub fn append_message(mut self, msg: impl Into<ChatMessage>) -> Self {
		self.messages.to_mut().push(msg.into());
		self
	}

	/// Append multiple messages from any iterable.
	pub fn append_messages<I>(mut self, messages: I) -> Self
	where
		I: IntoIterator,
		I::Item: Into<ChatMessage>,
	{
		self.messages.to_mut().extend(messages.into_iter().map(Into::into));
		self
	}

	/// Replace the tool set.
	pub fn with_tools(mut self, tools: impl Into<Tools<'tools>>) -> Self {
		self.tools = Some(tools.into());
		self
	}

	/// Set the previous response ID for stateful sessions.
	pub fn with_previous_response_id(mut self, previous_response_id: impl Into<String>) -> Self {
		self.previous_response_id = Some(previous_response_id.into());
		self
	}

	/// Set whether to store the response for stateful sessions.
	pub fn with_store(mut self, store: bool) -> Self {
		self.store = Some(store);
		self
	}

	/// Append one tool.
	pub fn append_tool(mut self, tool: impl Into<Tool>) -> Self {
		self.tools.get_or_insert_with(Default::default).to_mut().push(tool.into());
		self
	}

	/// Appends a captured assistant tool-use turn followed by its tool response.
	///
	/// Captured content is preferred because it preserves part ordering. If no assistant
	/// content or tool calls were captured, only the tool response is appended.
	pub fn append_tool_use_from_stream_end(mut self, end: &StreamEnd, tool_response: ToolResponse) -> Self {
		if let Some(content) = &end.captured_content {
			// Use captured content directly. Older adapters may still expose reasoning
			// only through the separately normalized convenience field.
			let mut assistant = ChatMessage::assistant(content.clone());
			if !content.contains_reasoning_content() {
				assistant = assistant.with_reasoning_content(end.captured_reasoning_content.clone());
			}
			self.messages.to_mut().push(assistant);
		} else if let Some(calls_ref) = end.captured_tool_calls() {
			// Fallback: build assistant message from tool calls only
			let calls: Vec<ToolCall> = calls_ref.into_iter().cloned().collect();
			if !calls.is_empty() {
				self.messages
					.to_mut()
					.push(ChatMessage::from(calls).with_reasoning_content(end.captured_reasoning_content.clone()));
			}
		}

		// Append the tool response turn
		self.messages.to_mut().push(ChatMessage::from(tool_response));
		self
	}
}

/// Getters
impl ChatRequest<'_, '_> {
	/// Iterate over all system content: the top-level system prompt, then any system-role messages.
	pub fn iter_systems(&self) -> impl Iterator<Item = &str> {
		self.system
			.iter()
			.map(|s| s.as_str())
			.chain(self.messages.iter().filter_map(|message| match message.role {
				ChatRole::System => message.content.first_text(),
				_ => None,
			}))
	}

	/// Joins all system content with one empty line between entries.
	pub fn join_systems(&self) -> Option<String> {
		let mut systems: Option<String> = None;

		for system in self.iter_systems() {
			let systems_content = systems.get_or_insert_with(String::new);

			support::combine_text_with_empty_line(systems_content, system);
		}

		systems
	}

	#[deprecated(note = "use join_systems()")]
	/// Deprecated alias for [`Self::join_systems`].
	pub fn combine_systems(&self) -> Option<String> {
		self.join_systems()
	}
}

impl From<Vec<ChatMessage>> for ChatRequest<'_, '_> {
	fn from(messages: Vec<ChatMessage>) -> Self {
		Self {
			system: None,
			messages: messages.into(),
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}
}

// endregion: --- ChatRequest
