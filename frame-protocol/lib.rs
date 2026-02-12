use std::io::Read;

pub fn write_frame<W: std::io::Write>(writer: &mut W, message: &str) -> std::io::Result<usize> {
    let size = u16::try_from(message.len());
    if let Ok(size) = size {
        let _ = writer.write(&size.to_le_bytes())?;
        writer.write(message.as_bytes())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::FileTooLarge,
            "frame-protocol supports a maximum frame size of 65536",
        ))
    }
}

pub struct FrameReader<R> {
    reader: R,
}

impl<R> FrameReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: Read> Iterator for FrameReader<R> {
    type Item = std::io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut marker_buf = [0u8; 2];
        let result = self.reader.read_exact(&mut marker_buf);
        if let Err(err) = result {
            return if let std::io::ErrorKind::UnexpectedEof = err.kind() {
                None
            } else {
                Some(Err(err))
            };
        };

        let size = u16::from_le_bytes(marker_buf);
        let mut buf = String::with_capacity(size.into());
        let result = self
            .reader
            .by_ref()
            .take(size.into())
            .read_to_string(&mut buf);

        match result {
            Ok(_) => Some(Ok(buf)),
            Err(err) => Some(Err(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write() {
        let mut buffer: Vec<u8> = Vec::new();

        write_frame(&mut buffer, "Hello World").unwrap();
        write_frame(&mut buffer, "Test").unwrap();

        let out: Result<Vec<String>, _> = FrameReader::new(buffer.as_slice()).collect();
        let out: Vec<String> = out.unwrap();

        assert_eq!(out, &["Hello World".to_string(), "Test".to_string()]);
    }
}
