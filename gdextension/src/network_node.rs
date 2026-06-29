use godot::prelude::*;

use wasteland_network::connection::{ConnectionManager, PeerId};
use wasteland_network::frame::{Frame, MessageType};
use wasteland_network::lockstep::LockstepState;
use wasteland_network::rollback::RollbackBuffer;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandNetwork {
    #[var]
    tick_rate: i64,
    #[var]
    max_rollback_frames: i64,
    #[var]
    input_delay: i64,
    #[var]
    is_host: bool,

    connection: ConnectionManager,
    #[allow(dead_code)]
    lockstep: LockstepState,
    rollback: RollbackBuffer,
    frame_count: i64,
    sent_bytes: i64,
    received_bytes: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandNetwork {
    fn init(base: Base<Node>) -> Self {
        Self {
            tick_rate: 60,
            max_rollback_frames: 8,
            input_delay: 2,
            is_host: false,
            connection: ConnectionManager::new(PeerId(0), 32),
            lockstep: LockstepState::new(2, 32),
            rollback: RollbackBuffer::new(),
            frame_count: 0,
            sent_bytes: 0,
            received_bytes: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandNetwork {
    #[func]
    fn create_frame(
        &mut self,
        msg_type: GString,
        sender_id: i64,
        payload: PackedByteArray,
    ) -> Dictionary<Variant, Variant> {
        let mt = match msg_type.to_string().as_str() {
            "input" => MessageType::Input,
            "state" => MessageType::State,
            "ack" => MessageType::Ack,
            "sync" => MessageType::Sync,
            "heartbeat" => MessageType::Heartbeat,
            _ => MessageType::Custom(0x80),
        };
        let frame =
            Frame::new(self.frame_count as u64, mt, sender_id as u32, payload.as_slice().to_vec());
        self.frame_count += 1;
        self.sent_bytes += frame.payload.len() as i64;
        dict! {
            "frame_number" => frame.frame_number as i64,
            "sender_id" => frame.sender_id as i64,
            "payload_size" => frame.payload.len() as i64,
            "checksum" => frame.checksum as i64,
        }
    }

    #[func]
    fn validate_frame(&self, _frame_number: i64, payload: PackedByteArray, _checksum: i64) -> bool {
        let hash = payload.as_slice().iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32));
        hash == _checksum as u32
    }

    #[func]
    fn connect_to(&mut self, _address: GString, _port: i64) -> bool {
        self.connection.add_peer(PeerId(1)).is_ok()
    }

    #[func]
    fn disconnect(&mut self) {
        self.connection.remove_peer(PeerId(1));
    }

    #[func]
    fn is_connected(&self) -> bool {
        self.connection.connected_count() > 0
    }

    #[func]
    fn get_connection_stats(&self) -> Dictionary<Variant, Variant> {
        let peer_count = self.connection.peer_count();
        dict! {
            "connected" => self.connection.connected_count() > 0,
            "peer_count" => peer_count as i64,
            "ping_ms" => 0.0,
            "packet_loss_percent" => 0.0,
        }
    }

    #[func]
    fn push_rollback_state(&mut self, frame: i64, state_data: PackedByteArray) {
        self.rollback.save_state(frame as u64, state_data.as_slice().to_vec());
    }

    #[func]
    fn get_rollback_state(&mut self, frame: i64) -> PackedByteArray {
        let data = self.rollback.rollback_to(frame as u64);
        match data {
            Some(d) => {
                let mut arr = PackedByteArray::new();
                for &b in d.state_data.iter() {
                    arr.push(b);
                }
                arr
            },
            None => PackedByteArray::new(),
        }
    }

    #[func]
    fn clear_rollback_history(&mut self, _before_frame: i64) {
        self.rollback.clear_old_states(0);
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "frame_count" => self.frame_count,
            "sent_bytes" => self.sent_bytes,
            "received_bytes" => self.received_bytes,
            "tick_rate" => self.tick_rate,
            "is_host" => self.is_host,
        }
    }
}
