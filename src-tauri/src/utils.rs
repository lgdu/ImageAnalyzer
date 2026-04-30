use std::fs;

pub fn read_file_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Failed to read file: {}", e))
}

pub fn bytes_to_hex(bytes: &[u8], max_len: usize) -> String {
    let limited = if bytes.len() > max_len {
        &bytes[..max_len]
    } else {
        bytes
    };
    limited
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
