use conn::TcpChunker;
use myo_proto::xfer::types::Packet;
use myo_proto::xfer::wwr::WwrState;

pub fn handle_packet_in(packet: Packet, state: &mut WwrState, conn: &mut TcpChunker) {
    state.handle_ack(&packet.ack);
    if conn.can_send() && packet.chunk.is_some() {
        let mut buffer = Vec::new();
        let mut finished = false;
        for chunk in state.handle_chunk(packet.chunk.unwrap()) {
            if chunk.data.len() == 0 {
                finished = true;
                // Data past EOF is meaningless.
                break;
            } else {
                buffer.extend(chunk.data);
            }
        }
        if buffer.len() > 0 {
            conn.send(buffer);
        }
        if finished {
            conn.send_finished();
        }
    }
}

pub fn next_packet_out(state: &mut WwrState, conn: &mut TcpChunker) -> Packet {
    while state.send_buffer_space() > 0 {
        if let Some(data) = conn.recv() {
            state.push_send_buffer(data);
        } else {
            break;
        }
    }
    Packet{
        ack: state.next_send_ack(),
        chunk: state.next_send_chunk()
    }
}
