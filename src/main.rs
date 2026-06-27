use tokio::net::UdpSocket;
use std::os::windows::fs::FileExt;
use std::{error::Error, io, net::SocketAddr};
use std::fs;
struct Server {
    socket: UdpSocket,
    buf: [u8; 10_000],
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

        loop {
            if let Some((size, client)) = self.to_send {
                // write to a new jpeg file on disk.
                let new_file_path = format!("src/images/image_{}.jpeg", image_count);
                let file = fs::OpenOptions::new().write(true).create(true).open(new_file_path).unwrap();
                let mut current_offset = 0;
                println!("got some bytes: {}", size);
                println!("writing bytes...");
                for chunk in self.buf[..size].chunks(8192) {
                    file.seek_write(chunk, current_offset).unwrap();
                    let amount_of_bytes_sent = self.socket.send_to(chunk, &client).await?;
                    // echo it back
                    current_offset += chunk.len() as u64;
                }
                println!("done writing bytes.");
                image_count += 1;
                // clear buffer after use/re-initialize
                self.buf.fill(0);
            }
            // set to_send when something is received
            self.to_send = Some(self.socket.recv_from(&mut self.buf).await?);
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
        buf: [0_u8; 10_000],
        to_send: None,
    };

    server.run().await?;

    Ok(())
}
