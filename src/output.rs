use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct OutputHandler {
    stdout: StandardStream,
}

impl OutputHandler {
    pub fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Always),
        }
    }

    pub fn print_user_message(&mut self, content: &str) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
        write!(self.stdout, "You: ")?;
        self.stdout.reset()?;
        writeln!(self.stdout, "{}", content)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_ai_message(&mut self, content: &str) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(self.stdout, "ARULA: ")?;
        self.stdout.reset()?;
        writeln!(self.stdout, "{}", content)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_error(&mut self, content: &str) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        write!(self.stdout, "Error: ")?;
        self.stdout.reset()?;
        writeln!(self.stdout, "{}", content)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_system(&mut self, content: &str) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(self.stdout, "{}", content)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_tool_call(&mut self, name: &str, args: &str) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
        write!(self.stdout, "🔧 Tool: ")?;
        self.stdout.reset()?;
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
        writeln!(self.stdout, "{}", name)?;
        if !args.is_empty() {
            writeln!(self.stdout, "   Args: {}", args)?;
        }
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_tool_result(&mut self, result: &str) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
        writeln!(self.stdout, "   Result: {}", result)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_streaming_chunk(&mut self, chunk: &str) -> io::Result<()> {
        write!(self.stdout, "{}", chunk)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn start_ai_message(&mut self) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(self.stdout, "ARULA: ")?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn end_line(&mut self) -> io::Result<()> {
        writeln!(self.stdout)?;
        self.stdout.flush()?;
        Ok(())
    }

    pub fn print_banner(&mut self) -> io::Result<()> {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
        writeln!(self.stdout, "╔═══════════════════════════════════════╗")?;
        writeln!(self.stdout, "║      ARULA - Autonomous AI CLI        ║")?;
        writeln!(self.stdout, "╚═══════════════════════════════════════╝")?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }
}

impl Default for OutputHandler {
    fn default() -> Self {
        Self::new()
    }
}
