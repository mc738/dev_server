use sha1::{Digest, Sha1};

/// Handle the WebSockets handshake and return a WebSockets key for use in the Sec-WebSocket-Accept
//  http header.
pub fn handle_handshake(key: &String) -> String {
    let mut hasher = Sha1::new();

    hasher.update(format!("{}{}", key, "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes());

    base64::encode(hasher.finalize())
}

/// Handle creating a short WebSocket message to be send to a client.
pub fn handle_write(data: &mut Vec<u8>, length: u8) -> Vec<u8> {
    let mut response = Vec::with_capacity(length as usize + 2);
    let fin: u8 = 0x80;
    let byte1 = fin | 1;
    let byte2: u8 = 0 | length;

    response.push(byte1);
    response.push(byte2);

    response.append(data);
    response
}
