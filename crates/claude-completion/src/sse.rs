//! Persistent line-buffered SSE parser for the Anthropic Messages API.
//!
//! Anthropic emits named events (`event: <name>\ndata: <json>\n\n`). The
//! `type` discriminator on the JSON payload also tells us the event name,
//! so the parser does not strictly need the `event:` line for routing —
//! but the spec allows it and we capture it for forward-compat tracing.
//!
//! The parser is fed raw HTTP chunks via [`SseParser::feed`]; for every
//! complete `data:` line it deserializes the JSON payload and invokes the
//! caller-supplied closure. Malformed lines are logged and skipped.
//! A hard ceiling (`MAX_LINE_BUFFER_SIZE`) prevents a misbehaving peer
//! from holding bytes without a newline forever.
//!
//! Internals: the buffer is `Vec<u8>` (not `String`) so multi-byte UTF-8
//! characters that straddle a chunk boundary are preserved. Each
//! newline-delimited line is decoded once via `String::from_utf8_lossy`,
//! which only inserts U+FFFD for genuinely invalid sequences inside the
//! complete line — never for valid sequences split across chunks.

use astrid_sdk::prelude::*;

use crate::schemas::StreamingEvent;

/// Hard ceiling on the line-accumulator. 256 KiB matches the per-spec
/// expectation: a single SSE field value (data:`<json>`) for a Messages
/// API chunk is comfortably under this.
pub(crate) const MAX_LINE_BUFFER_SIZE: usize = 256 * 1024;

/// Stateful, persistent SSE parser. Owns the line buffer and the
/// last-seen `event:` name across `feed` calls.
pub(crate) struct SseParser {
    /// Accumulator for partial lines between chunks. Bytes, not chars,
    /// so a multi-byte UTF-8 sequence split across two chunks survives.
    buffer: Vec<u8>,
    /// Most recently observed `event:` field, cleared on each blank line.
    /// Kept for diagnostics; routing actually uses the JSON `type` tag.
    current_event: String,
}

impl SseParser {
    /// Create a fresh parser.
    pub(crate) fn new() -> Self {
        Self {
            buffer: Vec::new(),
            current_event: String::new(),
        }
    }

    /// Feed a chunk of bytes from the HTTP stream. Invokes `on_event` for
    /// every successfully parsed `StreamingEvent`. Returns `Err` only when
    /// the line buffer overflows or `on_event` itself errors.
    pub(crate) fn feed<F>(&mut self, chunk: &[u8], mut on_event: F) -> Result<(), SysError>
    where
        F: FnMut(&str, StreamingEvent) -> Result<(), SysError>,
    {
        self.buffer.extend_from_slice(chunk);

        if self.buffer.len() > MAX_LINE_BUFFER_SIZE {
            return Err(SysError::ApiError(
                "SSE line buffer exceeded maximum size".into(),
            ));
        }

        while let Some(newline_pos) = self.buffer.iter().position(|b| *b == b'\n') {
            // Split off the line bytes (without the trailing `\n`).
            let mut line_bytes: Vec<u8> = self.buffer.drain(..=newline_pos).collect();
            // Pop the trailing `\n`.
            line_bytes.pop();
            // Trim a single trailing `\r` if present (CRLF normalisation).
            if line_bytes.last() == Some(&b'\r') {
                line_bytes.pop();
            }

            // Decode once now that the line is complete. `from_utf8_lossy`
            // here is safe — any U+FFFD it inserts is for genuinely invalid
            // UTF-8 inside a complete line, not for chunk-boundary splits.
            let raw_line = String::from_utf8_lossy(&line_bytes);

            if raw_line.is_empty() {
                // Blank line = end of event; reset the named-event tracker.
                self.current_event.clear();
                continue;
            }

            if let Some(name) = raw_line.strip_prefix("event: ") {
                self.current_event = name.to_string();
                continue;
            }

            let Some(data) = raw_line.strip_prefix("data: ") else {
                // Comments (`: …`), `id:`, `retry:` — ignored.
                continue;
            };

            match serde_json::from_str::<StreamingEvent>(data) {
                Ok(event) => on_event(&self.current_event, event)?,
                Err(e) => {
                    // Forward-compat: Anthropic may add event types before
                    // we know them. Don't tear down the stream.
                    log::warn(format!(
                        "claude-completion: skipping unparseable SSE data line ({e}): {data}"
                    ));
                }
            }
        }

        Ok(())
    }
}
