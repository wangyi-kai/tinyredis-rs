use crate::parser::frame::Frame;

use bytes::{Buf, BufMut, BytesMut};
use std::io::{self, Cursor, Write};

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tracing::{debug, info};

#[derive(Debug)]
pub struct Connection {
    pub(crate) stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(4 * 1024),
        }
    }

    pub async fn read_frame(&mut self) -> crate::Result<Option<Frame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                debug!("read frame [{:?}]", frame);
                return Ok(Some(frame))
            }
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    Err("connection reset by peer".into())
                }
            }
        }
    }

    fn parse_frame(&mut self) -> crate::Result<Option<Frame>> {
        use crate::parser::frame::Error::Incomplete;

        let mut buf = Cursor::new(&self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);
                let frame = Frame::parse(&mut buf)?;
                self.buffer.advance(len);
                Ok(Some(frame))
            }
            Err(Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        debug!("write frame [{:?}]", frame);
        let mut bytes = vec![];
        Self::write_value(frame, &mut bytes);
        self.stream.write_all(bytes.as_mut_slice()).await?;
        self.stream.flush().await
    }

    pub fn write_value(frame: &Frame, bytes: &mut Vec<u8>) {
        match frame {
            Frame::Simple(data) => {
                bytes.extend_from_slice(b"+");
                bytes.extend_from_slice(data.as_bytes());
                bytes.extend_from_slice(b"\r\n");
            }
            Frame::Error(data) => {
                bytes.extend_from_slice(b"-");
                bytes.extend_from_slice(data.as_bytes());
                bytes.extend_from_slice(b"\r\n");
            }
            Frame::Integer(data) => {
                bytes.extend_from_slice(b":");
                bytes.extend_from_slice(data.to_string().as_bytes());
                bytes.extend_from_slice(b"\r\n");
            }
            Frame::Bulk(data) => {
                bytes.extend_from_slice(b"$");
                bytes.extend_from_slice(data.len().to_string().as_bytes());
                bytes.extend_from_slice(b"\r\n");
                bytes.extend_from_slice(data);
                bytes.extend_from_slice(b"\r\n");
            }
            Frame::Null => {
                bytes.extend_from_slice(b"$-1\r\n");
            }
            Frame::Array(data) => {
                bytes.extend_from_slice(b"*");
                bytes.extend_from_slice(data.len().to_string().as_bytes());
                bytes.extend_from_slice(b"\r\n");
                for item in data {
                    Self::write_value(item, bytes);
                }
            }
        }
    }
}