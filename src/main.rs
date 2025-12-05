use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "UltimaDL")]
#[command(about = "High-performance stream-based YouTube downloader/converter")]
struct Cli {
    /// The YouTube URL
    url: String,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Mode::Mp4)]
    mode: Mode,

    /// Output filename (without extension)
    #[arg(short, long, default_value = "output")]
    name: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    Mp3,
    Mp4,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!(">>> Starting Stream: {} -> {}.{}", cli.url, cli.name, format!("{:?}", cli.mode).to_lowercase());

    // 1. Configure yt-dlp command
    let mut ytdlp_cmd = Command::new("yt-dlp");
    
    // Base arguments
    ytdlp_cmd.args([
        "--output", "-",      // Output to stdout
        "--quiet",            // Reduce noise
        "--progress",         // Show standard progress
        "--extractor-args", "youtube:player_client=default", // Fix missing JS warning
    ]);

    // OPTIMIZATION: Select specific streams to save bandwidth
    match cli.mode {
        Mode::Mp3 => {
            // ONLY download audio. Huge bandwidth saving.
            ytdlp_cmd.args(["-f", "bestaudio/best"]);
        },
        Mode::Mp4 => {
            // Download video + audio
            ytdlp_cmd.args(["-f", "bestvideo+bestaudio/best"]);
        },
    }

    // Add URL last
    ytdlp_cmd.arg(&cli.url);

    // Spawn the downloader process
    let mut ytdlp_child = ytdlp_cmd
        .stdout(Stdio::piped())   // Create the pipe
        .spawn()
        .context("Failed to start yt-dlp. Is it installed?")?;

    let ytdlp_stdout = ytdlp_child.stdout.take().expect("Failed to capture stdout");

    // 2. Configure ffmpeg to read from STDIN (pipe:0)
    let mut ffmpeg_cmd = Command::new("ffmpeg");
    ffmpeg_cmd
        .args(["-y", "-i", "pipe:0"]) // Read from pipe
        .stdin(ytdlp_stdout);         // Connect the pipe

    match cli.mode {
        Mode::Mp3 => {
            // Audio only, high quality mp3
            ffmpeg_cmd.args(["-vn", "-acodec", "libmp3lame", "-q:a", "2", &format!("{}.mp3", cli.name)]);
        },
        Mode::Mp4 => {
            // Video copy + Audio copy -> fast
            ffmpeg_cmd.args(["-c", "copy", "-movflags", "+faststart", &format!("{}.mp4", cli.name)]);
        },
    }

    // 3. Execute the pipeline
    let mut ffmpeg_child = ffmpeg_cmd
        .spawn()
        .context("Failed to start ffmpeg. Is it installed?")?;

    println!(">>> Streaming data... (Ctrl+C to cancel)");

    let status = ffmpeg_child.wait()?;

    if status.success() {
        println!("\n>>> Success! Saved to {}.{}", cli.name, format!("{:?}", cli.mode).to_lowercase());
    } else {
        eprintln!("\n>>> Process failed.");
    }

    Ok(())
}
