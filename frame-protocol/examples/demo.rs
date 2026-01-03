use std::io::{self, Result as IOResult, Write};

fn main() -> IOResult<()> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    frame_protocol::write_frame(&mut stdout, "Hello World")?;
    stdout.flush().unwrap();

    let mut reader = frame_protocol::FrameReader::new(stdin);
    let input = reader.next().unwrap()?;

    frame_protocol::write_frame(&mut stdout, &format!("recieved: {input}"))?;

    Ok(())
}
