//! Markdown streaming support for real-time AI response rendering
//! Based on codex-rs markdown_stream implementation

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use std::collections::VecDeque;

/// Markdown stream processor that handles incremental markdown rendering
pub struct MarkdownStream {
    /// Buffered content waiting to be rendered
    buffer: String,
    /// Completed lines ready for display
    completed_lines: Vec<Line<'static>>,
    /// Current line being built
    current_line: Line<'static>,
    /// Whether we're in a code block
    in_code_block: bool,
    /// Code block language
    code_block_lang: String,
    /// Whether we're in a list
    in_list: bool,
    /// Current list depth
    list_depth: usize,
    /// Whether we're in a blockquote
    in_blockquote: bool,
}

impl Default for MarkdownStream {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownStream {
    /// Create a new markdown stream processor
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            completed_lines: Vec::new(),
            current_line: Line::default(),
            in_code_block: false,
            code_block_lang: String::new(),
            in_list: false,
            list_depth: 0,
            in_blockquote: false,
        }
    }

    /// Push new content from the AI stream
    pub fn push(&mut self, content: &str) -> Vec<Line<'static>> {
        self.buffer.push_str(content);
        self.process_buffer()
    }

    /// Finalize the stream and return any remaining content
    pub fn finalize(&mut self) -> Vec<Line<'static>> {
        let mut result = Vec::new();

        // Process any remaining content
        result.extend(self.process_buffer());

        // Add the current line if it has content
        if !self.current_line.spans.is_empty() {
            result.push(self.current_line.clone());
            self.current_line = Line::default();
        }

        result
    }

    /// Clear all buffered content
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.completed_lines.clear();
        self.current_line = Line::default();
        self.in_code_block = false;
        self.code_block_lang.clear();
        self.in_list = false;
        self.list_depth = 0;
        self.in_blockquote = false;
    }

    /// Process the buffer and return completed lines
    fn process_buffer(&mut self) -> Vec<Line<'static>> {
        let mut result = Vec::new();

        // Process complete lines
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line_content = self.buffer[..newline_pos].to_string();
            self.buffer = self.buffer[newline_pos + 1..].to_string();

            // Process the line
            if let Some(line) = self.process_line(&line_content) {
                result.push(line);
            }
        }

        result
    }

    /// Process a single line of markdown
    fn process_line(&mut self, line: &str) -> Option<Line<'static>> {
        let line = line.trim_end();

        // Check for code block delimiter
        if line.starts_with("```") {
            self.in_code_block = !self.in_code_block;
            if self.in_code_block {
                self.code_block_lang = line[3..].to_string();
            } else {
                self.code_block_lang.clear();
            }
            return None;
        }

        // Skip processing while in code block
        if self.in_code_block {
            return Some(Line::styled(
                line.to_string(),
                Style::default().fg(Color::Rgb(180, 180, 180)),
            ));
        }

        // Check for headers
        if line.starts_with('#') {
            return self.process_header(line);
        }

        // Check for blockquote
        if line.starts_with('>') {
            self.in_blockquote = true;
            return Some(Line::styled(
                line[1..].trim().to_string(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        // Check for list items
        if self.is_list_item(line) {
            return Some(Line::styled(
                line.to_string(),
                Style::default().fg(Color::White),
            ));
        }

        // Regular text
        Some(Line::styled(line.to_string(), Style::default().fg(Color::White)))
    }

    /// Process a markdown header
    fn process_header(&mut self, line: &str) -> Option<Line<'static>> {
        let hashes = line.chars().take_while(|&c| c == '#').count();
        let content = line[hashes..].trim();

        let style = match hashes {
            1 => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            2 => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            3 => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            _ => Style::default().fg(Color::Green),
        };

        Some(Line::styled(content.to_string(), style))
    }

    /// Check if a line is a list item
    fn is_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || trimmed.chars().next().map_or(false, |c| c.is_ascii_digit())
                && trimmed.chars().nth(1) == Some('.')
    }
}

/// Streaming markdown renderer with inline formatting
pub struct StreamingMarkdown {
    stream: MarkdownStream,
    pending_chunks: VecDeque<String>,
}

impl Default for StreamingMarkdown {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingMarkdown {
    /// Create a new streaming markdown renderer
    pub fn new() -> Self {
        Self {
            stream: MarkdownStream::new(),
            pending_chunks: VecDeque::new(),
        }
    }

    /// Add a chunk of markdown content from the stream
    pub fn add_chunk(&mut self, chunk: &str) -> Vec<Line<'static>> {
        let clean = self.clean_ansi(chunk);
        self.stream.push(&clean)
    }

    /// Finalize the stream and return any remaining lines
    pub fn finalize(&mut self) -> Vec<Line<'static>> {
        self.stream.finalize()
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.stream.clear();
        self.pending_chunks.clear();
    }

    /// Clean ANSI escape codes from content
    fn clean_ansi(&self, s: &str) -> String {
        use console::strip_ansi_codes;
        use regex::Regex;
        use std::sync::OnceLock;

        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(r"(\x1b\[[0-9;]*[A-Za-z]|\[\d{1,3}(?:;\d{1,3})*m)").unwrap()
        });

        let stripped = strip_ansi_codes(s);
        re.replace_all(&stripped, "").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_stream_basic() {
        let mut stream = MarkdownStream::new();
        let lines = stream.push("Hello\n");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_markdown_stream_finalize() {
        let mut stream = MarkdownStream::new();
        stream.push("Hello");
        let lines = stream.finalize();
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_markdown_stream_code_block() {
        let mut stream = MarkdownStream::new();
        stream.push("```rust\n");
        stream.push("let x = 1;\n");
        stream.push("```\n");
        let lines = stream.finalize();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_markdown_stream_headers() {
        let mut stream = MarkdownStream::new();
        let lines = stream.push("# Header 1\n");
        assert_eq!(lines.len(), 1);
    }
}
