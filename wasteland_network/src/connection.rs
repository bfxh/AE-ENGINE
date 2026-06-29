use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Syncing,
    Connected,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerId(pub u32);

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: PeerId,
    pub state: ConnectionState,
    pub latency_ms: f32,
    pub last_ack_frame: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_lost: u64,
}

impl PeerInfo {
    pub fn new(id: PeerId) -> Self {
        PeerInfo {
            id,
            state: ConnectionState::Disconnected,
            latency_ms: 0.0,
            last_ack_frame: 0,
            packets_sent: 0,
            packets_received: 0,
            packets_lost: 0,
        }
    }

    pub fn packet_loss_rate(&self) -> f32 {
        if self.packets_sent == 0 {
            return 0.0;
        }
        self.packets_lost as f32 / self.packets_sent as f32
    }
}

#[allow(dead_code)]
pub struct ConnectionManager {
    peers: HashMap<PeerId, PeerInfo>,
    local_id: PeerId,
    max_peers: usize,
    heartbeat_interval_ms: u64,
    timeout_ms: u64,
}

impl ConnectionManager {
    pub fn new(local_id: PeerId, max_peers: usize) -> Self {
        ConnectionManager {
            peers: HashMap::new(),
            local_id,
            max_peers,
            heartbeat_interval_ms: 1000,
            timeout_ms: 5000,
        }
    }

    pub fn add_peer(&mut self, id: PeerId) -> Result<(), String> {
        if self.peers.len() >= self.max_peers {
            return Err("max peers reached".into());
        }
        if self.peers.contains_key(&id) {
            return Err("peer already exists".into());
        }
        self.peers.insert(id, PeerInfo::new(id));
        Ok(())
    }

    pub fn remove_peer(&mut self, id: PeerId) {
        self.peers.remove(&id);
    }

    pub fn get_peer(&self, id: PeerId) -> Option<&PeerInfo> {
        self.peers.get(&id)
    }

    pub fn get_peer_mut(&mut self, id: PeerId) -> Option<&mut PeerInfo> {
        self.peers.get_mut(&id)
    }

    pub fn set_state(&mut self, id: PeerId, state: ConnectionState) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.state = state;
        }
    }

    pub fn update_latency(&mut self, id: PeerId, latency_ms: f32) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.latency_ms = peer.latency_ms * 0.9 + latency_ms * 0.1;
        }
    }

    pub fn record_send(&mut self, id: PeerId) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.packets_sent += 1;
        }
    }

    pub fn record_receive(&mut self, id: PeerId) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.packets_received += 1;
        }
    }

    pub fn record_loss(&mut self, id: PeerId) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.packets_lost += 1;
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn connected_count(&self) -> usize {
        self.peers.values().filter(|p| p.state == ConnectionState::Connected).count()
    }

    pub fn check_timeouts(&mut self, current_time_ms: u64) -> Vec<PeerId> {
        let mut timed_out = Vec::new();
        for peer in self.peers.values_mut() {
            if peer.state == ConnectionState::Connected {
                timed_out.push(peer.id);
                peer.state = ConnectionState::Timeout;
            }
        }
        let _ = current_time_ms;
        timed_out
    }

    pub fn all_peers(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_peer() {
        let mut mgr = ConnectionManager::new(PeerId(0), 4);
        mgr.add_peer(PeerId(1)).unwrap();
        mgr.add_peer(PeerId(2)).unwrap();
        assert_eq!(mgr.peer_count(), 2);
        mgr.remove_peer(PeerId(1));
        assert_eq!(mgr.peer_count(), 1);
    }

    #[test]
    fn test_add_duplicate_peer() {
        let mut mgr = ConnectionManager::new(PeerId(0), 4);
        mgr.add_peer(PeerId(1)).unwrap();
        assert!(mgr.add_peer(PeerId(1)).is_err());
    }

    #[test]
    fn test_max_peers() {
        let mut mgr = ConnectionManager::new(PeerId(0), 2);
        mgr.add_peer(PeerId(1)).unwrap();
        mgr.add_peer(PeerId(2)).unwrap();
        assert!(mgr.add_peer(PeerId(3)).is_err());
    }

    #[test]
    fn test_latency_smoothing() {
        let mut mgr = ConnectionManager::new(PeerId(0), 4);
        mgr.add_peer(PeerId(1)).unwrap();
        mgr.update_latency(PeerId(1), 100.0);
        assert!((mgr.get_peer(PeerId(1)).unwrap().latency_ms - 10.0).abs() < 0.1);
        mgr.update_latency(PeerId(1), 100.0);
        assert!((mgr.get_peer(PeerId(1)).unwrap().latency_ms - 19.0).abs() < 0.1);
    }

    #[test]
    fn test_packet_loss_rate() {
        let mut mgr = ConnectionManager::new(PeerId(0), 4);
        mgr.add_peer(PeerId(1)).unwrap();
        mgr.record_send(PeerId(1));
        mgr.record_send(PeerId(1));
        mgr.record_send(PeerId(1));
        mgr.record_loss(PeerId(1));
        let rate = mgr.get_peer(PeerId(1)).unwrap().packet_loss_rate();
        assert!((rate - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_connected_count() {
        let mut mgr = ConnectionManager::new(PeerId(0), 4);
        mgr.add_peer(PeerId(1)).unwrap();
        mgr.add_peer(PeerId(2)).unwrap();
        mgr.set_state(PeerId(1), ConnectionState::Connected);
        mgr.set_state(PeerId(2), ConnectionState::Syncing);
        assert_eq!(mgr.connected_count(), 1);
    }
}
