use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error, io, net::SocketAddr};
struct Server {
    socket: Arc<Mutex<UdpSocket>>,
    buf: [u8; 50000],
    to_send: Option<(usize, SocketAddr)>,
}
struct VideoViewerWantsToWatchCurrentFootage {

}
impl Server {
    async fn run(&mut self) -> Result<(), io::Error> {
        let mut ffmpeg_video_writer = Command::new("ffmpeg")
            .args([
                // inputs
                "-f", "image2pipe",
                "-vcodec", "mjpeg",
                "-r", "14",
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
        let mut stdin = ffmpeg_video_writer.stdin.take().expect("Failed to open stdin to ffmpeg");
        let mut frame_buffer = Vec::new();
        loop {
            self.to_send = Some(self.socket.lock().await.recv_from(&mut self.buf).await?);

            if let Some((size, client)) = self.to_send {
                let message = String::from_utf8_lossy(&self.buf[.."video_viewer_watch".len()]); 
                println!("got message. {:?}", String::from_utf8_lossy(&self.buf[..15]));
                // spawn task to pipe mkv => rgb565be output to send chunks
                if message == "video_viewer_watch" {
                    println!("ITS A VIDEO_VIEWER_WATCH,  CREATING TASK TO PIPE RGB565 TO VIDEO VIEWER");
                    let socket1 = self.socket.clone();
                    tokio::spawn(async move {
                        let mut ffmpeg_video_viewer_rgb565_output = Command::new("ffmpeg")
                            .args([
                                // inputs
                                "-i", "src/videos/output.mkv",
                                "-f", "rawvideo",
                                "-pix_fmt", "rgb565be",
                                // "-r", "14",
                                "pipe:1",
                            ])
                            .stdin(Stdio::null())
                            .stderr(Stdio::inherit())
                            .stdout(Stdio::piped())
                            .spawn()
                            .unwrap();
                        let mut stdout = ffmpeg_video_viewer_rgb565_output.stdout.take().expect("Failed to open stdin to ffmpeg");
                        let mut rgb565_frame = [0_u8; 153_600]; 
                        loop {
                            // read_exact essentially is like clearing the buffer
                            stdout.read_exact(&mut rgb565_frame).unwrap();
                            // screen has to draw a proper rectangle at a time, and not just
                            // iterate from left to right pixels i think... so 320widthx30height

                            // doesnt seem to be sending to them. pico recv_from is getting
                            // nothing...
                            for chunk in rgb565_frame.chunks(9600) {
                                println!("SENDING CHUNK--------------------...");
                                let _ = socket1.lock().await.send_to(chunk, client).await;
                            }
                        }
                    });
                }
                // write to mkv file
                else if size == 50000 {
                    frame_buffer.extend_from_slice(&self.buf[..size]);
                    self.buf.fill(0);
                }
                // ignore
                else if size > 50000 {
                    println!("GREATER THAN 50000 BYTE PACKET DETECTED, DROPPING");
                }
                // full jpeg write to mkv file or finish writing jpeg to mkv file
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let addr = "0.0.0.0:6767".to_string();

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on {}...", socket.local_addr()?);

    let mut server = Server {
        socket: Arc::new(Mutex::new(socket)),
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
