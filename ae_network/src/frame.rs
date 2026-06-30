use serde::{Deserialize, Serialize};

pub const MAX_FRAME_SIZE: usize = 1200;
pub const HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    Input = 0x01,
    State = 0x02,
    Ack = 0x03,
    Sync = 0x04,
    Heartbeat = 0x05,
    Custom(u8),
}

impl MessageType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(MessageType::Input),
            0x02 => Some(MessageType::State),
            0x03 => Some(MessageType::Ack),
            0x04 => Some(MessageType::Sync),
            0x05 => Some(MessageType::Heartbeat),
            n if n >= 0x80 => Some(MessageType::Custom(n)),
            _ => None,
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            MessageType::Input => 0x01,
            MessageType::State => 0x02,
            MessageType::Ack => 0x03,
            MessageType::Sync => 0x04,
            MessageType::Heartbeat => 0x05,
            MessageType::Custom(n) => n,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub frame_number: u64,
    pub msg_type: MessageType,
    pub sender_id: u32,
    pub payload: Vec<u8>,
    pub checksum: u32,
}

impl Frame {
    pub fn new(frame_number: u64, msg_type: MessageType, sender_id: u32, payload: Vec<u8>) -> Self {
        let checksum = Frame::compute_checksum(frame_number, &payload);
        Frame { frame_number, msg_type, sender_id, payload, checksum }
    }

    fn compute_checksum(frame_number: u64, payload: &[u8]) -> u32 {
        let mut hash: u32 = 0x811c9dc5;
        for &b in frame_number.to_le_bytes().iter() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        for &b in payload {
            hash ^= b as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        hash
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&self.frame_number.to_le_bytes());
        buf.push(self.msg_type.to_byte());
        buf.push((self.payload.len() >> 8) as u8);
        buf.push((self.payload.len() & 0xff) as u8);
        buf.push(0);
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    pub fn decode(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < HEADER_SIZE {
            return Err("frame too short".into());
        }
        let frame_number = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let msg_type = MessageType::from_byte(data[8])
            .ok_or_else(|| format!("unknown message type: {}", data[8]))?;
        let payload_len = ((data[9] as usize) << 8) | (data[10] as usize);
        if data.len() < HEADER_SIZE + payload_len + 4 {
            return Err("frame truncated".into());
        }
        let payload = data[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec();
        let checksum = u32::from_le_bytes(
            data[HEADER_SIZE + payload_len..HEADER_SIZE + payload_len + 4].try_into().unwrap(),
        );
        let expected = Frame::compute_checksum(frame_number, &payload);
        if checksum != expected {
            return Err(format!("checksum mismatch: got {}, expected {}", checksum, expected));
        }
        Ok((
            Frame { frame_number, msg_type, sender_id: 0, payload, checksum },
            HEADER_SIZE + payload_len + 4,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueue {
    frames: Vec<Frame>,
    next_frame: u64,
}

impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue { frames: Vec::new(), next_frame: 0 }
    }

    pub fn push(&mut self, frame: Frame) {
        let pos = self
            .frames
            .binary_search_by_key(&frame.frame_number, |f| f.frame_number)
            .unwrap_or_else(|e| e);
        self.frames.insert(pos, frame);
    }

    pub fn pop(&mut self, frame_number: u64) -> Option<Frame> {
        if let Some(pos) = self.frames.iter().position(|f| f.frame_number == frame_number) {
            Some(self.frames.remove(pos))
        } else {
            None
        }
    }

    pub fn has_frame(&self, frame_number: u64) -> bool {
        self.frames.binary_search_by_key(&frame_number, |f| f.frame_number).is_ok()
    }

    pub fn pending_count(&self) -> usize {
        self.frames.len()
    }

    pub fn drain_before(&mut self, frame_number: u64) -> Vec<Frame> {
        let split = self.frames.partition_point(|f| f.frame_number < frame_number);
        self.frames.drain(..split).collect()
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_encode_decode() {
        let frame = Frame::new(42, MessageType::Input, 1, vec![10, 20, 30]);
        let encoded = frame.encode();
        let (decoded, size) = Frame::decode(&encoded).unwrap();
        assert_eq!(size, encoded.len());
        assert_eq!(decoded.frame_number, 42);
        assert_eq!(decoded.msg_type, MessageType::Input);
        assert_eq!(decoded.payload, vec![10, 20, 30]);
    }

    #[test]
    fn test_frame_checksum_detection() {
        let frame = Frame::new(1, MessageType::State, 0, vec![1, 2, 3]);
        let mut encoded = frame.encode();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xff;
        assert!(Frame::decode(&encoded).is_err());
    }

    #[test]
    fn test_message_queue_push_pop() {
        let mut queue = MessageQueue::new();
        queue.push(Frame::new(2, MessageType::Input, 0, vec![2]));
        queue.push(Frame::new(1, MessageType::Input, 0, vec![1]));
        assert_eq!(queue.pop(1).unwrap().payload, vec![1]);
        assert_eq!(queue.pop(2).unwrap().payload, vec![2]);
        assert!(queue.pop(3).is_none());
    }

    #[test]
    fn test_message_queue_has_frame() {
        let mut queue = MessageQueue::new();
        queue.push(Frame::new(5, MessageType::Input, 0, vec![]));
        assert!(queue.has_frame(5));
        assert!(!queue.has_frame(6));
    }

    #[test]
    fn test_drain_before() {
        let mut queue = MessageQueue::new();
        for i in 0..10 {
            queue.push(Frame::new(i, MessageType::Input, 0, vec![]));
        }
        let drained = queue.drain_before(5);
        assert_eq!(drained.len(), 5);
        assert_eq!(queue.pending_count(), 5);
    }
}
