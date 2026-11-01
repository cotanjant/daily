use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use clap::{Parser, Subcommand};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{QueueableCommand, cursor};
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "daily", version, about = "Everyday tools for your terminal")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download a file from an HTTP or HTTPS URL.
    Download {
        url: String,
        path: PathBuf,
        /// Replace an existing file without asking.
        #[arg(short, long)]
        yes: bool,
    },
    /// Wait until a time, then show a desktop notification.
    Remind {
        message: String,
        /// RFC 3339 timestamp, such as 2026-11-01T18:30:00+03:00.
        #[arg(long)]
        at: String,
    },
    /// Everyday filesystem operations.
    File {
        #[command(subcommand)]
        action: FileCommands,
    },
    /// Date and duration helpers.
    Time {
        #[command(subcommand)]
        action: TimeCommands,
    },
}

#[derive(Subcommand)]
enum FileCommands {
    /// Copy a file to a destination.
    Copy {
        source: PathBuf,
        destination: PathBuf,
        #[arg(short, long)]
        yes: bool,
    },
    /// Move a file to a destination.
    Move {
        source: PathBuf,
        destination: PathBuf,
        #[arg(short, long)]
        yes: bool,
    },
    /// Rename a file.
    Rename {
        path: PathBuf,
        new_path: PathBuf,
        #[arg(short, long)]
        yes: bool,
    },
    /// Create a directory and missing parents.
    Mkdir { path: PathBuf },
}

#[derive(Subcommand)]
enum TimeCommands {
    /// Format an RFC 3339 timestamp using chrono/strftime directives.
    Format { value: String, format: String },
    /// Parse a datetime string and interpret it as UTC.
    Parse { value: String, format: String },
    /// Format a number of seconds as a compact duration.
    Duration { seconds: u64 },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Download { url, path, yes } => download(&url, &path, yes),
        Commands::Remind { message, at } => remind(&message, &at),
        Commands::File { action } => file_command(action),
        Commands::Time { action } => time_command(action),
    }
}

fn confirm_overwrite(path: &Path, yes: bool) -> Result<bool> {
    if !path.exists() || yes {
        return Ok(true);
    }
    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        bail!(
            "destination exists: {} (pass --yes to replace it)",
            path.display()
        );
    }
    let mut stdout = io::stdout();
    stdout
        .queue(SetForegroundColor(Color::Yellow))?
        .queue(Print("File exists: "))?
        .queue(SetForegroundColor(Color::Cyan))?
        .queue(Print(path.display().to_string()))?
        .queue(SetForegroundColor(Color::Magenta))?
        .queue(Print("\nOverwrite? (y/n) > "))?
        .queue(ResetColor)?
        .flush()?;
    loop {
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                print!("Please answer y or n > ");
                io::stdout().flush()?;
            }
        }
    }
}

fn progress(
    stdout: &mut io::Stdout,
    received: u64,
    total: Option<u64>,
    speed: f64,
) -> io::Result<()> {
    const WIDTH: usize = 30;
    stdout.queue(cursor::MoveToColumn(0))?;
    if let Some(total) = total.filter(|n| *n > 0) {
        let percent = (received.saturating_mul(100) / total).min(100) as usize;
        let filled = percent * WIDTH / 100;
        stdout
            .queue(Print("["))?
            .queue(SetForegroundColor(Color::Cyan))?
            .queue(Print("#".repeat(filled)))?
            .queue(SetForegroundColor(Color::DarkGrey))?
            .queue(Print("-".repeat(WIDTH - filled)))?
            .queue(ResetColor)?
            .queue(Print(format!(
                "] {percent:3}%  {:.2}/{:.2} MB  {:.2} MB/s",
                received as f64 / 1e6,
                total as f64 / 1e6,
                speed
            )))?;
    } else {
        stdout
            .queue(SetForegroundColor(Color::Yellow))?
            .queue(Print(format!(
                "{:.2} MB  {:.2} MB/s",
                received as f64 / 1e6,
                speed
            )))?
            .queue(ResetColor)?;
    }
    stdout.queue(Clear(ClearType::UntilNewLine))?.flush()
}

fn download(url: &str, path: &Path, yes: bool) -> Result<()> {
    if !confirm_overwrite(path, yes)? {
        println!("Download cancelled.");
        return Ok(());
    }
    let mut response = reqwest::blocking::get(url)
        .map_err(|error| {
            anyhow::anyhow!(
                "download could not connect: {error}. Check your internet connection and try again."
            )
        })?
        .error_for_status()?;
    let temp = path.with_file_name(format!(
        ".daily-{}-{}.part",
        std::process::id(),
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .with_context(|| format!("creating temporary file {}", temp.display()))?;
    let total = response.content_length();
    let show = io::stdout().is_terminal();
    let mut stdout = io::stdout();
    let mut received = 0u64;
    let mut sample_time = Instant::now();
    let mut sample_bytes = 0u64;
    let mut speed = 0.0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = match response.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(error) => {
                drop(file);
                let _ = fs::remove_file(&temp);
                let message = format!(
                    "Download interrupted. The partial file was removed. Check your internet connection and try again. ({error})"
                );
                if show {
                    eprintln!("\x1b[31m{message}\x1b[0m");
                }
                bail!("{message}");
            }
        };
        if let Err(error) = file.write_all(&buffer[..count]) {
            drop(file);
            let _ = fs::remove_file(&temp);
            return Err(error.into());
        }
        received += count as u64;
        if show && sample_time.elapsed() >= Duration::from_millis(100) {
            let elapsed = sample_time.elapsed().as_secs_f64().max(0.001);
            speed = (received - sample_bytes) as f64 / elapsed / 1e6;
            let _ = progress(&mut stdout, received, total, speed);
            sample_bytes = received;
            sample_time = Instant::now();
        }
    }
    if total.is_some_and(|expected| received < expected) {
        drop(file);
        let _ = fs::remove_file(&temp);
        bail!(
            "Download interrupted before the full file arrived. The partial file was removed. Check your internet connection and try again."
        );
    }
    if let Err(error) = file.flush() {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    drop(file);
    if show {
        let _ = progress(&mut stdout, received, total, speed);
        println!();
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temp, path).with_context(|| format!("saving download to {}", path.display()))?;
    Ok(())
}

fn remind(message: &str, at: &str) -> Result<()> {
    let when = DateTime::parse_from_rfc3339(at)
        .context("--at must be an RFC 3339 timestamp with a timezone")?;
    let delay = (when.with_timezone(&Utc) - Utc::now())
        .to_std()
        .context("reminder time must be in the future")?;
    println!(
        "Reminder set for {}. Keep this process running until it fires.",
        when.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %Z")
    );
    thread::sleep(delay);
    #[cfg(target_os = "linux")]
    Command::new("notify-send")
        .args(["Reminder", message])
        .status()
        .context("showing notification; install notify-send")?;
    #[cfg(target_os = "macos")]
    Command::new("osascript")
        .args([
            "-e",
            &format!("display notification {:?} with title \"Reminder\"", message),
        ])
        .status()
        .context("showing notification")?;
    #[cfg(target_os = "windows")]
    {
        let escaped = message.replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $n=New-Object System.Windows.Forms.NotifyIcon; $n.Icon=[System.Drawing.SystemIcons]::Information; $n.BalloonTipTitle='Reminder'; $n.BalloonTipText='{escaped}'; $n.Visible=$true; $n.ShowBalloonTip(10000); Start-Sleep -Seconds 11; $n.Dispose()"
        );
        Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .status()
            .context("showing notification")?;
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    println!("Reminder: {message}");
    Ok(())
}

fn file_command(action: FileCommands) -> Result<()> {
    match action {
        FileCommands::Copy {
            source,
            destination,
            yes,
        } => {
            if confirm_overwrite(&destination, yes)? {
                fs::copy(source, &destination)?;
            }
        }
        FileCommands::Move {
            source,
            destination,
            yes,
        } => {
            if confirm_overwrite(&destination, yes)? {
                if destination.exists() {
                    fs::remove_file(&destination)?;
                }
                fs::rename(source, &destination)?;
            }
        }
        FileCommands::Rename {
            path,
            new_path,
            yes,
        } => {
            if confirm_overwrite(&new_path, yes)? {
                if new_path.exists() {
                    fs::remove_file(&new_path)?;
                }
                fs::rename(path, new_path)?;
            }
        }
        FileCommands::Mkdir { path } => fs::create_dir_all(path)?,
    }
    Ok(())
}

fn time_command(action: TimeCommands) -> Result<()> {
    match action {
        TimeCommands::Format { value, format } => {
            let value =
                DateTime::parse_from_rfc3339(&value).context("expected RFC 3339 datetime")?;
            println!("{}", value.format(&format));
        }
        TimeCommands::Parse { value, format } => {
            let value = NaiveDateTime::parse_from_str(&value, &format)
                .context("could not parse datetime")?;
            println!("{}", value.and_utc().to_rfc3339());
        }
        TimeCommands::Duration { seconds } => {
            let days = seconds / 86400;
            let hours = seconds % 86400 / 3600;
            let minutes = seconds % 3600 / 60;
            let secs = seconds % 60;
            let mut parts = Vec::new();
            if days > 0 {
                parts.push(format!("{days}d"));
            }
            if hours > 0 {
                parts.push(format!("{hours}h"));
            }
            if minutes > 0 {
                parts.push(format!("{minutes}m"));
            }
            if secs > 0 || parts.is_empty() {
                parts.push(format!("{secs}s"));
            }
            println!("{}", parts.join(" "));
        }
    }
    Ok(())
}
