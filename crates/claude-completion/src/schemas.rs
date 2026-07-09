//! Anthropic Messages API SSE event shapes.
//!
//! These types deserialize from the `data:` payloads on the Messages API
//! streaming response and are converted into `astrid_sdk` `StreamEvent`
//! variants by `sse.rs` / `anthropic.rs`.

use serde::Deserialize;
use serde_json::Value;

/// Top-level SSE event from the Anthropic Messages API.
///
/// The `data:` payload carries a `type` discriminator. The `event: <name>`
/// SSE line is redundant for routing but the parser still tracks it for
/// forward-compat (e.g. future event types Anthropic adds before the SDK
/// learns about them).
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum StreamingEvent {
    /// Sent at the start of a new message; carries initial usage.
    MessageStart {
        /// Partial message object (we only read `usage`).
        message: Value,
    },
    /// New content block begins (text or tool_use).
    ContentBlockStart {
        /// Block index within the message.
        #[expect(dead_code)]
        index: usize,
        /// Block definition.
        content_block: ContentBlock,
    },
    /// Incremental content within a block.
    ContentBlockDelta {
        /// Block index within the message.
        #[expect(dead_code)]
        index: usize,
        /// The incremental delta.
        delta: Delta,
    },
    /// Content block finished.
    ContentBlockStop {
        /// Block index within the message.
        #[expect(dead_code)]
        index: usize,
    },
    /// Message-level metadata change (cumulative output usage lives here).
    MessageDelta {
        /// Delta payload (stop_reason etc — currently unused).
        #[expect(dead_code)]
        delta: Value,
        /// Cumulative usage at end of message.
        usage: Option<AnthropicUsage>,
    },
    /// Whole message complete; terminator.
    MessageStop,
    /// Keep-alive; ignored.
    Ping,
    /// API error during streaming.
    Error {
        /// Error details, shape `{ "type": "<code>", "message": "..." }`.
        error: AnthropicError,
    },
}

/// A content block in the Anthropic streaming response.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ContentBlock {
    /// Text block. Initial text usually empty (streamed via `text_delta`).
    Text {
        /// Initial text (typically empty).
        #[expect(dead_code)]
        text: String,
    },
    /// Tool-use block.
    ToolUse {
        /// Tool call ID (forwarded as `StreamEvent::ToolCallStart.id`).
        id: String,
        /// Tool name.
        name: String,
        /// Partial input — usually empty `{}` for streaming; real args arrive
        /// via subsequent `input_json_delta`s.
        #[expect(dead_code)]
        input: Value,
    },
    /// Thinking block (extended thinking enabled). We don't surface
    /// reasoning yet; deltas appear as `ThinkingDelta` and are dropped.
    Thinking {
        /// Initial thinking text.
        #[expect(dead_code)]
        thinking: String,
    },
}

/// Incremental delta within a content block.
///
/// Variant names mirror Anthropic's wire-protocol `type` discriminator
/// verbatim (`text_delta`, `input_json_delta`, etc.) — the
/// `enum_variant_names` lint is suppressed because renaming would force
/// per-variant `#[serde(rename)]` annotations for zero benefit.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Delta {
    /// Text fragment.
    TextDelta {
        /// The text fragment.
        text: String,
    },
    /// Partial JSON for a tool's input. Forward opaquely; never parse
    /// per-chunk (mid-string `{` etc.).
    InputJsonDelta {
        /// Partial JSON to append to the call's accumulated args.
        partial_json: String,
    },
    /// Extended-thinking text fragment. Ignored for now.
    ThinkingDelta {
        /// The thinking fragment.
        #[expect(dead_code)]
        thinking: String,
    },
    /// Signature delta for a thinking block. Ignored.
    SignatureDelta {
        /// The signature value.
        #[expect(dead_code)]
        signature: String,
    },
}

/// Token-usage statistics from the Anthropic API.
///
/// `MessageStart.usage` carries `input_tokens` (and cache fields);
/// `MessageDelta.usage` carries the cumulative `output_tokens` at end of
/// stream. We emit one `StreamEvent::Usage` at the end combining both.
#[derive(Deserialize, Debug, Default)]
pub(crate) struct AnthropicUsage {
    /// Input tokens consumed.
    #[serde(default)]
    pub(crate) input_tokens: Option<usize>,
    /// Output tokens generated.
    #[serde(default)]
    pub(crate) output_tokens: Option<usize>,
}

/// Anthropic error payload (in-stream `error` events and HTTP error bodies).
#[derive(Deserialize, Debug)]
pub(crate) struct AnthropicError {
    /// Error type code (e.g. `overloaded_error`, `rate_limit_error`).
    #[serde(rename = "type")]
    pub(crate) error_type: String,
    /// Human-readable message.
    pub(crate) message: String,
}
