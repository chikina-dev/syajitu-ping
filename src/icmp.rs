pub const ICMP_HEADER_LEN: usize = 8;
pub const PAYLOAD_SIZE: usize = 32;
const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_ECHO_REPLY: u8 = 0;

#[derive(Clone, Copy, Debug)]
pub struct Reply {
    pub bytes: usize,
    pub ttl: u8,
    pub identifier: u16,
    pub sequence: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub bytes: usize,
    pub ttl: u8,
    pub identifier: u16,
    pub sequence: u16,
    pub payload: &'a [u8],
}

pub fn packet_size() -> usize {
    ICMP_HEADER_LEN + PAYLOAD_SIZE
}

pub fn build_echo_request(identifier: u16, sequence: u16) -> Vec<u8> {
    let payload: Vec<u8> = (0..PAYLOAD_SIZE).map(|index| index as u8).collect();
    build_echo_packet(ICMP_ECHO_REQUEST, identifier, sequence, &payload)
}

pub fn parse_echo_reply(buffer: &[u8]) -> Option<Reply> {
    let packet = parse_packet(buffer)?;
    (packet.kind == ICMP_ECHO_REPLY).then_some(Reply {
        bytes: packet.bytes,
        ttl: packet.ttl,
        identifier: packet.identifier,
        sequence: packet.sequence,
    })
}

pub fn parse_echo_request(buffer: &[u8]) -> Option<Request<'_>> {
    let packet = parse_packet(buffer)?;
    (packet.kind == ICMP_ECHO_REQUEST).then_some(Request {
        bytes: packet.bytes,
        ttl: packet.ttl,
        identifier: packet.identifier,
        sequence: packet.sequence,
        payload: packet.payload,
    })
}

pub fn build_echo_reply(identifier: u16, sequence: u16, payload: &[u8]) -> Vec<u8> {
    build_echo_packet(ICMP_ECHO_REPLY, identifier, sequence, payload)
}

#[derive(Clone, Copy)]
struct Packet<'a> {
    kind: u8,
    bytes: usize,
    ttl: u8,
    identifier: u16,
    sequence: u16,
    payload: &'a [u8],
}

fn build_echo_packet(kind: u8, identifier: u16, sequence: u16, payload: &[u8]) -> Vec<u8> {
    let mut packet = vec![0u8; ICMP_HEADER_LEN + payload.len()];
    packet[0] = kind;
    packet[1] = 0;
    packet[4..6].copy_from_slice(&identifier.to_be_bytes());
    packet[6..8].copy_from_slice(&sequence.to_be_bytes());
    packet[ICMP_HEADER_LEN..].copy_from_slice(payload);

    let checksum = checksum(&packet);
    packet[2..4].copy_from_slice(&checksum.to_be_bytes());
    packet
}

fn parse_packet(buffer: &[u8]) -> Option<Packet<'_>> {
    if let Some(packet) = parse_ip_packet(buffer) {
        return Some(packet);
    }

    parse_icmp_packet(buffer, 64)
}

fn parse_ip_packet(buffer: &[u8]) -> Option<Packet<'_>> {
    if buffer.len() < 20 || (buffer[0] >> 4) != 4 {
        return None;
    }

    let header_len = ((buffer[0] & 0x0f) as usize) * 4;
    if header_len < 20 || buffer.len() < header_len + ICMP_HEADER_LEN {
        return None;
    }

    let ttl = buffer[8];
    parse_icmp_packet(&buffer[header_len..], ttl)
}

fn parse_icmp_packet(buffer: &[u8], ttl: u8) -> Option<Packet<'_>> {
    if buffer.len() < ICMP_HEADER_LEN {
        return None;
    }

    Some(Packet {
        kind: buffer[0],
        bytes: buffer.len(),
        ttl,
        identifier: u16::from_be_bytes([buffer[4], buffer[5]]),
        sequence: u16::from_be_bytes([buffer[6], buffer[7]]),
        payload: &buffer[ICMP_HEADER_LEN..],
    })
}

fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;

    for chunk in bytes.chunks(2) {
        let word = match chunk {
            [high, low] => u16::from_be_bytes([*high, *low]) as u32,
            [high] => ((*high as u16) << 8) as u32,
            _ => 0,
        };
        sum += word;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_of_completed_packet_is_zero() {
        let packet = build_echo_request(0x1234, 7);
        assert_eq!(checksum(&packet), 0);
    }

    #[test]
    fn parse_bare_icmp_reply() {
        let payload: Vec<u8> = (0..PAYLOAD_SIZE).map(|index| index as u8).collect();
        let packet = build_echo_reply(0x1111, 3, &payload);

        let reply = parse_echo_reply(&packet).expect("reply should parse");
        assert_eq!(reply.identifier, 0x1111);
        assert_eq!(reply.sequence, 3);
    }

    #[test]
    fn parse_ipv4_wrapped_reply() {
        let payload: Vec<u8> = (0..PAYLOAD_SIZE).map(|index| index as u8).collect();
        let icmp = build_echo_reply(0x2222, 9, &payload);

        let mut packet = vec![0u8; 20 + icmp.len()];
        packet[0] = 0x45;
        packet[8] = 42;
        packet[20..].copy_from_slice(&icmp);

        let reply = parse_echo_reply(&packet).expect("reply should parse");
        assert_eq!(reply.identifier, 0x2222);
        assert_eq!(reply.sequence, 9);
        assert_eq!(reply.ttl, 42);
    }

    #[test]
    fn parse_bare_icmp_request() {
        let packet = build_echo_request(0xabcd, 5);

        let request = parse_echo_request(&packet).expect("request should parse");
        assert_eq!(request.identifier, 0xabcd);
        assert_eq!(request.sequence, 5);
        assert_eq!(request.payload.len(), PAYLOAD_SIZE);
    }

    #[test]
    fn reply_preserves_payload() {
        let request = build_echo_request(0x3333, 11);
        let parsed = parse_echo_request(&request).expect("request should parse");
        let reply = build_echo_reply(parsed.identifier, parsed.sequence, parsed.payload);
        let parsed_reply = parse_echo_reply(&reply).expect("reply should parse");

        assert_eq!(parsed_reply.identifier, 0x3333);
        assert_eq!(parsed_reply.sequence, 11);
        assert_eq!(reply[ICMP_HEADER_LEN..], request[ICMP_HEADER_LEN..]);
    }
}
