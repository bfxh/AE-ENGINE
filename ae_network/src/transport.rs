//! 网络传输层：基于 UDP 的数据包收发

use crate::connection::PeerId;
use std::collections::VecDeque;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

/// 传输层错误
#[derive(Debug)]
pub enum TransportError {
    Io(std::io::Error),
    SocketBind(String),
    SendFailed(String),
    RecvFailed(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::SocketBind(e) => write!(f, "socket bind failed: {e}"),
            Self::SendFailed(e) => write!(f, "send failed: {e}"),
            Self::RecvFailed(e) => write!(f, "recv failed: {e}"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// UDP 传输层
pub struct UdpTransport {
    socket: UdpSocket,
    recv_buffer: Vec<u8>,
    peer_addresses: hashbrown::HashMap<PeerId, SocketAddr>,
    address_to_peer: hashbrown::HashMap<SocketAddr, PeerId>,
    next_peer_id: u32,
    outgoing_queue: VecDeque<(PeerId, Vec<u8>)>,
    incoming_queue: VecDeque<(PeerId, Vec<u8>)>,
    stats: TransportStats,
}

/// 传输统计
#[derive(Debug, Clone, Default)]
pub struct TransportStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub send_errors: u64,
    pub recv_errors: u64,
}

impl UdpTransport {
    /// 绑定到指定地址
    pub fn bind(addr: &str) -> Result<Self, TransportError> {
        let socket = UdpSocket::bind(addr)
            .map_err(|e| TransportError::SocketBind(format!("{addr}: {e}")))?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            recv_buffer: vec![0u8; 65536],
            peer_addresses: hashbrown::HashMap::new(),
            address_to_peer: hashbrown::HashMap::new(),
            next_peer_id: 1,
            outgoing_queue: VecDeque::new(),
            incoming_queue: VecDeque::new(),
            stats: TransportStats::default(),
        })
    }

    /// 绑定到随机可用端口
    pub fn bind_any() -> Result<Self, TransportError> {
        Self::bind("0.0.0.0:0")
    }

    /// 获取本地绑定地址
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.socket.local_addr()?)
    }

    /// 注册远程 peer
    pub fn add_peer(&mut self, addr: SocketAddr) -> PeerId {
        if let Some(&id) = self.address_to_peer.get(&addr) {
            return id;
        }
        let id = PeerId(self.next_peer_id);
        self.next_peer_id += 1;
        self.peer_addresses.insert(id, addr);
        self.address_to_peer.insert(addr, id);
        id
    }

    /// 通过 PeerId 获取地址
    pub fn peer_address(&self, id: PeerId) -> Option<SocketAddr> {
        self.peer_addresses.get(&id).copied()
    }

    /// 通过地址获取 PeerId
    pub fn peer_id(&self, addr: &SocketAddr) -> Option<PeerId> {
        self.address_to_peer.get(addr).copied()
    }

    /// 入队待发送数据
    pub fn send_to(&mut self, peer: PeerId, data: Vec<u8>) {
        self.outgoing_queue.push_back((peer, data));
    }

    /// 广播到所有 peer
    pub fn broadcast(&mut self, data: Vec<u8>) {
        for &peer_id in self.peer_addresses.keys() {
            self.outgoing_queue.push_back((peer_id, data.clone()));
        }
    }

    /// 处理所有待发送数据
    pub fn flush(&mut self) -> Result<usize, TransportError> {
        let mut sent = 0;
        while let Some((peer, data)) = self.outgoing_queue.pop_front() {
            if let Some(addr) = self.peer_addresses.get(&peer) {
                match self.socket.send_to(&data, addr) {
                    Ok(n) => {
                        self.stats.bytes_sent += n as u64;
                        self.stats.packets_sent += 1;
                        sent += 1;
                    },
                    Err(e) => {
                        self.stats.send_errors += 1;
                        log::warn!("send to {} failed: {}", addr, e);
                    },
                }
            }
        }
        Ok(sent)
    }

    /// 接收所有可用数据包（非阻塞）
    pub fn poll(&mut self) -> Result<usize, TransportError> {
        let mut received = 0;
        loop {
            match self.socket.recv_from(&mut self.recv_buffer) {
                Ok((n, addr)) => {
                    let peer_id = if let Some(id) = self.address_to_peer.get(&addr) {
                        *id
                    } else {
                        // 自动注册新 peer
                        self.add_peer(addr)
                    };
                    self.incoming_queue.push_back((peer_id, self.recv_buffer[..n].to_vec()));
                    self.stats.bytes_received += n as u64;
                    self.stats.packets_received += 1;
                    received += 1;
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                },
                Err(e) => {
                    self.stats.recv_errors += 1;
                    return Err(TransportError::Io(e));
                },
            }
        }
        Ok(received)
    }

    /// 弹出一个接收到的数据包
    pub fn recv(&mut self) -> Option<(PeerId, Vec<u8>)> {
        self.incoming_queue.pop_front()
    }

    /// 获取接收队列长度
    pub fn incoming_count(&self) -> usize {
        self.incoming_queue.len()
    }

    /// 获取发送队列长度
    pub fn outgoing_count(&self) -> usize {
        self.outgoing_queue.len()
    }

    /// 获取统计信息
    pub fn stats(&self) -> &TransportStats {
        &self.stats
    }

    /// 设置 socket 阻塞模式
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), TransportError> {
        self.socket.set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// 设置读取超时
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.socket.set_read_timeout(timeout)?;
        Ok(())
    }

    /// 获取已注册的 peer 数量
    pub fn peer_count(&self) -> usize {
        self.peer_addresses.len()
    }

    /// 获取所有 peer ID
    pub fn peers(&self) -> Vec<PeerId> {
        self.peer_addresses.keys().copied().collect()
    }
}

/// 网络消息封装
#[derive(Debug, Clone)]
pub struct NetworkMessage {
    pub sequence: u64,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

impl NetworkMessage {
    pub fn new(sequence: u64, timestamp: u64, payload: Vec<u8>) -> Self {
        Self { sequence, timestamp, payload }
    }

    /// 序列化为字节
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + self.payload.len());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// 从字节反序列化
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }
        let sequence = u64::from_le_bytes(data[0..8].try_into().ok()?);
        let timestamp = u64::from_le_bytes(data[8..16].try_into().ok()?);
        let payload = data[16..].to_vec();
        Some(Self { sequence, timestamp, payload })
    }
}

/// 可靠传输配置
#[derive(Debug, Clone)]
pub struct ReliableConfig {
    pub max_retries: u32,
    pub retry_interval: Duration,
    pub ack_timeout: Duration,
    pub window_size: u32,
}

impl Default for ReliableConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            retry_interval: Duration::from_millis(100),
            ack_timeout: Duration::from_millis(500),
            window_size: 64,
        }
    }
}

/// 可靠传输层（简化版：ARQ + ACK）
pub struct ReliableTransport {
    config: ReliableConfig,
    pending_acks: hashbrown::HashMap<u64, (Instant, u32, Vec<u8>)>, // seq -> (sent_time, retries, data)
    received_seqs: hashbrown::HashMap<PeerId, u64>,                 // peer -> last received seq
    next_seq: u64,
}

impl ReliableTransport {
    pub fn new(config: ReliableConfig) -> Self {
        Self {
            config,
            pending_acks: hashbrown::HashMap::new(),
            received_seqs: hashbrown::HashMap::new(),
            next_seq: 1,
        }
    }

    /// 发送可靠消息
    pub fn send_reliable(&mut self, data: Vec<u8>) -> (u64, Vec<u8>) {
        let seq = self.next_seq;
        self.next_seq += 1;
        let msg = NetworkMessage::new(seq, current_timestamp_ms(), data);
        let serialized = msg.serialize();
        self.pending_acks.insert(seq, (Instant::now(), 0, serialized.clone()));
        (seq, serialized)
    }

    /// 处理接收到的消息，返回 (payload, needs_ack)
    pub fn receive(&mut self, peer: PeerId, data: &[u8]) -> Option<(Vec<u8>, u64)> {
        let msg = NetworkMessage::deserialize(data)?;
        let last_seq = self.received_seqs.get(&peer).copied().unwrap_or(0);
        if msg.sequence <= last_seq {
            // 重复消息，忽略
            return None;
        }
        self.received_seqs.insert(peer, msg.sequence);
        Some((msg.payload, msg.sequence))
    }

    /// 处理 ACK
    pub fn handle_ack(&mut self, seq: u64) {
        self.pending_acks.remove(&seq);
    }

    /// 生成 ACK 消息
    pub fn create_ack(seq: u64) -> Vec<u8> {
        // ACK 消息格式: [0xFF; 8] + seq
        let mut buf = vec![0xFFu8; 8];
        buf.extend_from_slice(&seq.to_le_bytes());
        buf
    }

    /// 检查是否是 ACK 消息
    pub fn is_ack(data: &[u8]) -> Option<u64> {
        if data.len() == 16 && data[..8] == [0xFF; 8] {
            return Some(u64::from_le_bytes(data[8..16].try_into().ok()?));
        }
        None
    }

    /// 检查超时并重发
    pub fn check_timeouts(&mut self) -> Vec<(u64, Vec<u8>)> {
        let mut resends = Vec::new();
        let retry_interval = self.config.retry_interval;
        let max_retries = self.config.max_retries;

        self.pending_acks.retain(|seq, (sent_time, retries, data)| {
            if sent_time.elapsed() >= retry_interval {
                if *retries >= max_retries {
                    log::warn!("reliable message {} timed out after {} retries", seq, retries);
                    return false; // 移除
                }
                *retries += 1;
                *sent_time = Instant::now();
                resends.push((*seq, data.clone()));
            }
            true
        });

        resends
    }

    /// 待确认消息数
    pub fn pending_count(&self) -> usize {
        self.pending_acks.len()
    }
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[test]
    fn udp_transport_bind() {
        let transport = UdpTransport::bind("127.0.0.1:0");
        assert!(transport.is_ok());
    }

    #[test]
    fn udp_transport_bind_any() {
        let transport = UdpTransport::bind_any();
        assert!(transport.is_ok());
    }

    #[test]
    fn udp_transport_local_addr() {
        let transport = UdpTransport::bind_any().unwrap();
        let addr = transport.local_addr();
        assert!(addr.is_ok());
    }

    #[test]
    fn udp_transport_add_peer() {
        let mut transport = UdpTransport::bind_any().unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 12345));
        let id = transport.add_peer(addr);
        assert_eq!(transport.peer_count(), 1);
        assert_eq!(transport.peer_address(id), Some(addr));
        assert_eq!(transport.peer_id(&addr), Some(id));
    }

    #[test]
    fn udp_transport_add_duplicate_peer() {
        let mut transport = UdpTransport::bind_any().unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 12345));
        let id1 = transport.add_peer(addr);
        let id2 = transport.add_peer(addr);
        assert_eq!(id1, id2);
        assert_eq!(transport.peer_count(), 1);
    }

    #[test]
    fn udp_transport_send_recv_queues() {
        let mut transport = UdpTransport::bind_any().unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 12345));
        let id = transport.add_peer(addr);

        transport.send_to(id, vec![1, 2, 3]);
        assert_eq!(transport.outgoing_count(), 1);
        assert_eq!(transport.incoming_count(), 0);
    }

    #[test]
    fn network_message_serialize_deserialize() {
        let msg = NetworkMessage::new(42, 1000, vec![1, 2, 3, 4]);
        let serialized = msg.serialize();
        assert_eq!(serialized.len(), 20); // 8 + 8 + 4

        let deserialized = NetworkMessage::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.sequence, 42);
        assert_eq!(deserialized.timestamp, 1000);
        assert_eq!(deserialized.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn network_message_deserialize_too_short() {
        let result = NetworkMessage::deserialize(&[1, 2, 3]);
        assert!(result.is_none());
    }

    #[test]
    fn reliable_transport_send() {
        let mut rt = ReliableTransport::new(ReliableConfig::default());
        let (seq, data) = rt.send_reliable(vec![1, 2, 3]);
        assert_eq!(seq, 1);
        assert!(!data.is_empty());
        assert_eq!(rt.pending_count(), 1);
    }

    #[test]
    fn reliable_transport_ack() {
        let mut rt = ReliableTransport::new(ReliableConfig::default());
        let (seq, _) = rt.send_reliable(vec![1, 2, 3]);
        assert_eq!(rt.pending_count(), 1);
        rt.handle_ack(seq);
        assert_eq!(rt.pending_count(), 0);
    }

    #[test]
    fn reliable_transport_receive() {
        let mut rt = ReliableTransport::new(ReliableConfig::default());
        let peer = PeerId(1);
        let msg = NetworkMessage::new(1, 1000, vec![1, 2, 3]);
        let data = msg.serialize();

        let result = rt.receive(peer, &data);
        assert!(result.is_some());
        let (payload, seq) = result.unwrap();
        assert_eq!(payload, vec![1, 2, 3]);
        assert_eq!(seq, 1);

        // 重复消息应被忽略
        let result2 = rt.receive(peer, &data);
        assert!(result2.is_none());
    }

    #[test]
    fn reliable_transport_ack_message() {
        let seq = 42u64;
        let ack = ReliableTransport::create_ack(seq);
        assert_eq!(ack.len(), 16);
        let parsed = ReliableTransport::is_ack(&ack);
        assert_eq!(parsed, Some(seq));
    }

    #[test]
    fn reliable_transport_not_ack() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let result = ReliableTransport::is_ack(&data);
        assert!(result.is_none());
    }

    #[test]
    fn transport_stats_default() {
        let stats = TransportStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
    }

    #[test]
    fn reliable_config_default() {
        let config = ReliableConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.window_size, 64);
    }
}
