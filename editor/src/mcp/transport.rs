use std::io::{self, BufRead, BufReader, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

pub trait McpTransport: Send {
    fn receive(&mut self) -> Option<String>;
    fn send(&mut self, message: &str) -> io::Result<()>;
}

pub struct StdioTransport {
    stdin: BufReader<io::Stdin>,
    stdout: io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        StdioTransport {
            stdin: BufReader::new(io::stdin()),
            stdout: io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl McpTransport for StdioTransport {
    fn receive(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.stdin.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            Err(_) => None,
        }
    }

    fn send(&mut self, message: &str) -> io::Result<()> {
        writeln!(self.stdout, "{}", message)?;
        self.stdout.flush()
    }
}

pub struct ChannelTransport {
    receiver: mpsc::Receiver<String>,
    sender: mpsc::Sender<String>,
}

impl ChannelTransport {
    pub fn new() -> (Self, mpsc::Sender<String>, mpsc::Receiver<String>) {
        let (tx_in, rx_in) = mpsc::channel();
        let (tx_out, rx_out) = mpsc::channel();
        (
            ChannelTransport {
                receiver: rx_in,
                sender: tx_out,
            },
            tx_in,
            rx_out,
        )
    }
}

impl McpTransport for ChannelTransport {
    fn receive(&mut self) -> Option<String> {
        self.receiver.try_recv().ok()
    }

    fn send(&mut self, message: &str) -> io::Result<()> {
        self.sender
            .send(message.to_string())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

pub struct MemoryTransport {
    messages: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<Vec<String>>>,
}

/// Handle to a `MemoryTransport` that can be held by external code (e.g. an
/// MCP skill, an HTTP bridge, or a test) to push JSON-RPC requests into the
/// editor and read the responses back. The handle is cheap to clone and
/// fully `Send + Sync`.
#[derive(Clone)]
pub struct MemoryTransportHandle {
    messages: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<Vec<String>>>,
}

impl MemoryTransportHandle {
    /// Push a JSON-RPC request line into the transport's inbox. The editor's
    /// MCP server will pick this up on the next `poll()`.
    pub fn push_message(&self, msg: &str) {
        self.messages.lock().unwrap().push(msg.to_string());
    }

    /// Pop the oldest response the MCP server has written. Responses are
    /// queued in order, so callers should drain this in a loop.
    pub fn pop_response(&self) -> Option<String> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            None
        } else {
            Some(responses.remove(0))
        }
    }

    /// Drain all pending responses into a single Vec (oldest first).
    pub fn drain_responses(&self) -> Vec<String> {
        let mut responses = self.responses.lock().unwrap();
        std::mem::take(&mut *responses)
    }

    /// Number of pending requests still in the inbox.
    pub fn pending_request_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// Number of pending responses waiting to be read.
    pub fn pending_response_count(&self) -> usize {
        self.responses.lock().unwrap().len()
    }
}

impl MemoryTransport {
    pub fn new() -> Self {
        MemoryTransport {
            messages: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a transport and a handle that shares its inbox/outbox.
    /// The handle lets external code push requests and read responses
    /// without having access to the `McpTransport` trait object (which
    /// is owned by the `McpServer` as `Box<dyn McpTransport>`).
    pub fn new_with_handle() -> (Self, MemoryTransportHandle) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(Vec::new()));
        let transport = MemoryTransport {
            messages: messages.clone(),
            responses: responses.clone(),
        };
        let handle = MemoryTransportHandle { messages, responses };
        (transport, handle)
    }

    pub fn push_message(&self, msg: &str) {
        self.messages.lock().unwrap().push(msg.to_string());
    }

    pub fn pop_response(&self) -> Option<String> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            None
        } else {
            Some(responses.remove(0))
        }
    }
}

impl Default for MemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl McpTransport for MemoryTransport {
    fn receive(&mut self) -> Option<String> {
        let mut msgs = self.messages.lock().unwrap();
        if msgs.is_empty() {
            None
        } else {
            Some(msgs.remove(0))
        }
    }

    fn send(&mut self, message: &str) -> io::Result<()> {
        self.responses.lock().unwrap().push(message.to_string());
        Ok(())
    }
}
