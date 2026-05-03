use std::fmt::Write;

pub fn format_hex_dump(bytes: &[u8]) -> String {
    let mut output = String::new();

    for (line_index, chunk) in bytes.chunks(16).enumerate() {
        let offset = line_index * 16;
        let _ = write!(&mut output, "{offset:04x}: ");

        for index in 0..16 {
            if let Some(byte) = chunk.get(index) {
                let _ = write!(&mut output, "{byte:02x} ");
            } else {
                output.push_str("   ");
            }
        }

        output.push(' ');

        for byte in chunk {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            output.push(ch);
        }

        if offset + chunk.len() < bytes.len() {
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_single_line_dump() {
        let dump = format_hex_dump(b"ABC");
        assert!(dump.contains("0000: 41 42 43"));
        assert!(dump.ends_with("ABC"));
    }
}
