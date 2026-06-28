use tokio::net::UdpSocket;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{error::Error, io, net::SocketAddr};
struct Server {
    socket: UdpSocket,
    buf: [u8; 50000],
    to_send: Option<(usize, SocketAddr)>,
}
impl Server {
    async fn run(&mut self) -> Result<(), io::Error> {
        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                // inputs
                "-f", "image2pipe",
                "-vcodec", "mjpeg",
                "-r", "20",
                "-i", "-",

                // outputs
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-y",
                "src/videos/output.mkv",
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = ffmpeg.stdin.take().expect("Failed to open stdin to ffmpeg");
        let mut frame_buffer = Vec::new();
        loop {
            self.to_send = Some(self.socket.recv_from(&mut self.buf).await?);

            if let Some((size, client)) = self.to_send {
                if size == 50000 {
                        frame_buffer.extend_from_slice(&self.buf[..size]);
                        self.buf.fill(0);
                }
                else if size > 50000 {
                    println!("GREATER THAN 50000 BYTE PACKET DETECTED, DROPPING");
                }
                else {
                    frame_buffer.extend_from_slice(&self.buf[..size]);
                    self.buf.fill(0);
                    if let Err(e) = stdin.write_all(&frame_buffer) {
                        eprintln!("Failed to write frame to FFmpeg: {}", e);
                    }
                    stdin.flush().unwrap();
                    frame_buffer.clear();
                }
            }
        }
        // drop(stdin);
        // ffmpeg.wait().unwrap();
    }
}

async fn convert_mp4_to_rgb565_for_video_viewer_task() {

}
async fn append_available_frames_to_mp4_task() {

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let addr = "0.0.0.0:6767".to_string();

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on {}...", socket.local_addr()?);

    let mut server = Server {
        socket,
        buf: [0_u8; 50000],
        to_send: None,
    };
    // 2. convert mp4 video to rgb565 for video_viewer, in 320wx240h format
    // tokio::spawn(async move {
    //     loop {
    //     }
    // });


    server.run().await?;

    Ok(())
}
