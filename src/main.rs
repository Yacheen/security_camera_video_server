use tokio::net::UdpSocket;
use std::os::windows::fs::FileExt;
use std::{error::Error, io, net::SocketAddr};
use std::fs::{self, File};
struct Server {
    socket: UdpSocket,
    buf: [u8; 40_000],
    to_send: Option<(usize, SocketAddr)>,
}
impl Server {
    async fn run(&mut self) -> Result<(), io::Error> {
        // let Server {
        //     socket,
        //     mut buf,
        //     mut to_send,
        // } = self;
        let mut image_count = 0;

        
        let writing_jpeg = false;
        let mut current_offset: u64 = 0;
        let mut current_filepath: Option<String> = None;
        let mut file: Option<File> = None;
        
        loop {
            // start loop when I receive a buffer with the first two bytes being 0xFF 0xD8
            // continuously receive until last two bytes indicate end of a jpeg file, 0xFF 0xD9
            self.to_send = Some(self.socket.recv_from(&mut self.buf).await?);

            if let Some((size, client)) = self.to_send {
                println!("byte size: {}", size);
                if size == 40000 {
                    // includes just start,
                    // includes neither start nor end
                    if self.buf[0] == 0xFF && self.buf[1] == 0xD8 {
                        current_filepath = Some(format!("src/images/image_{}.jpeg", image_count));
                        file = Some(fs::OpenOptions::new().write(true).create(true).open(current_filepath.clone().unwrap()).unwrap());
                    }
                    if let Some(file) = &file {
                        file.seek_write(&self.buf[..size], current_offset).unwrap();
                        self.buf.fill(0);
                        current_offset += 40000;
                    }
                }
                else if size > 40000 {
                    println!("GREATER THAN 40000 BYTE PACKET DETECTED, DROPPING");
                }
                else {
                    // includes start, end
                    // includes just end
                    // write to a new jpeg file on disk.
                    if let (Some(current_filepath), Some(file)) = (&current_filepath, &file) {
                        file.seek_write(&self.buf[..size], current_offset).unwrap();
                        println!("done writing bytes.");
                        image_count += 1;
                        // clear buffer since done writing jpeg.
                        self.buf.fill(0);
                    }
                    current_filepath = None;
                    file = None;
                    current_offset = 0;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let addr = "0.0.0.0:6767".to_string();

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on {}...", socket.local_addr()?);

    let mut server = Server {
        socket,
        buf: [0_u8; 40_000],
        to_send: None,
    };

    server.run().await?;

    Ok(())
}
