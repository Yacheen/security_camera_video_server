use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, };
use tokio::join;
use tokio::net::{TcpListener, UdpSocket};
use tokio::process::Command;
use std::io::{Read, Write};
use std::process::Stdio;
// use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error};
// tcp stream for streaming video to video viewer
// must be in order

// UDP for security camera video stream piped to stdin, output.mkv
// TCP for video viewer feed of outputmkv converted to rgb565 and piped to stdout, from output.mkv
struct Server {
    // udp_socket: Arc<UdpSocket>,
    // udp_buf: [u8; 50000],
    bla: u8,
}   
static UDP_ADDR: &str = "0.0.0.0:6767";
static TCP_ADDR: &str = "0.0.0.0:6769";
impl Server {
    async fn run(&mut self) -> Result<(), io::Error> {
        // tcp stream loop, for video viewer
        let tcp_handle = tokio::spawn(async move {
            let listener = TcpListener::bind(TCP_ADDR).await.unwrap();
            loop {
                let (mut socket, socket_addr) =  listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut tcp_buf: [u8; 50000] = [0; 50000];
                    match socket.read(&mut tcp_buf).await {
                         Ok(bytes_read) => {
                            let message = String::from_utf8_lossy(&tcp_buf[.."video_viewer_watch".len()]); 
                            if message == "video_viewer_watch" {
                                println!("video viewer watch requested. ip addr of client: {:?}", socket_addr.ip());
                                tokio::spawn(async move {
                                    let mut ffmpeg_video_viewer_rgb565_output = Command::new("ffmpeg")
                                        .args([
                                            // inputs
                                            // "-c:v", "rawvideo",
                                            // "-b:v", "1536000k",

                                            "-i", "src/videos/output.mkv",
                                            // outputs
                                            "-f", "rawvideo",
                                            "-vf", "scale=320:240",
                                            "-fps_mode", "passthrough",
                                            "-pix_fmt", "rgb565be",
                                            "-"
                                        ])
                                        .stdin(Stdio::null())
                                        .stderr(Stdio::inherit())
                                        .stdout(Stdio::piped())
                                        .spawn()
                                        .unwrap();
                                    let mut stdout = ffmpeg_video_viewer_rgb565_output.stdout.take().expect("Failed to open stdin to ffmpeg");
                                    let mut rgb565_frame = [0_u8; 153_600]; 

                                    let status = ffmpeg_video_viewer_rgb565_output.wait().await.unwrap();
                                    println!("security footage finished.");

                                    loop {
                                        match stdout.read_exact(&mut rgb565_frame).await {
                                            Ok(idk) => {
                                                // write in chunks
                                                for chunk in rgb565_frame.chunks(51200) {
                                                    let _ = socket.write_all(chunk).await;
                                                }
                                                // tokio::time::sleep(Duration::from_millis(100)).await;
                                            }
                                            Err(err) => {
                                                // println!("problem reading rgb565 frame: {:?}", err);
                                            }
                                        }
                                    }
                                });
                            }
                         }
                         Err(err) => {
                             println!("Problem reading tcp stream: {:?}", err);
                         }
                    };
                });
            }
        });

        // udp socket loop, for security camera, just streams the bytes into mp4 file unless bigger
        // than 5KB
        let udp_handle = tokio::spawn(async move {
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

            let udp_socket = UdpSocket::bind(UDP_ADDR).await.unwrap();
            let mut udp_buf: [u8; 50000] = [0; 50000];
            // for holding all of a jpeg
            let mut frame_buffer = Vec::new();
            loop {
                let (size, socket_addr) = udp_socket.recv_from(&mut udp_buf).await.unwrap();
                if size == 50000 {
                    println!("frame is 50kilobytes, writing full chunk");
                    frame_buffer.extend_from_slice(&udp_buf[..size]);
                    udp_buf.fill(0);
                }
                // ignore
                else if size > 50000 {
                    println!("GREATER THAN 50000 BYTE PACKET DETECTED, DROPPING");
                }
                // full jpeg write to mkv file or finish writing jpeg to mkv file
                else {
                    frame_buffer.extend_from_slice(&udp_buf[..size]);
                    udp_buf.fill(0);
                    if let Err(e) = stdin.write_all(&frame_buffer).await {
                        eprintln!("Failed to write frame to FFmpeg: {}", e);
                    }
                    let _ = stdin.flush().await;
                    frame_buffer.clear();
                }
            }
        });
        let (_tcp_res, _udp_res) = join!(tcp_handle, udp_handle);
        // drop(stdin);
        // ffmpeg.wait().unwrap();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let mut server = Server {
        bla: 0
    };

    server.run().await?;

    Ok(())
}
