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
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
        /// Future RFC 3339 timestamp with a timezone.
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

struct TempFile {
    path: PathBuf,
    cleanup: bool,
}

impl TempFile {
    fn create_near(destination: &Path) -> Result<(Self, fs::File)> {
        for _ in 0..32 {
            let path = temporary_sibling_path(destination, "part");
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok((
                        Self {
                            path,
                            cleanup: true,
                        },
                        file,
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("creating temporary file {}", path.display()));
                }
            }
        }
        bail!(
            "could not create a temporary file next to {}",
            destination.display()
        )
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(mut self) {
        self.cleanup = false;
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = remove_temporary_path(&self.path);
        }
    }
}

fn remove_temporary_path(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) => {
            #[cfg(windows)]
            if error.kind() == io::ErrorKind::PermissionDenied {
                let mut permissions = fs::metadata(path)?.permissions();
                permissions.set_readonly(false);
                fs::set_permissions(path, permissions)?;
                return fs::remove_file(path);
            }
            Err(error)
        }
    }
}

fn temporary_sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let id = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(".daily-{}-{id}.{suffix}", std::process::id()))
}

fn destination_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {
            bail!("destination is a directory: {}", path.display())
        }
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("checking {}", path.display())),
    }
}

fn same_path(first: &Path, second: &Path) -> bool {
    first == second
        || matches!(
            (fs::canonicalize(first), fs::canonicalize(second)),
            (Ok(first), Ok(second)) if first == second
        )
}

fn install_file(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(windows)]
    if destination_exists(destination)? {
        let backup = temporary_sibling_path(destination, "backup");
        fs::rename(destination, &backup).with_context(|| {
            format!("preserving the existing file at {}", destination.display())
        })?;
        if let Err(error) = fs::rename(source, destination) {
            if let Err(restore_error) = fs::rename(&backup, destination) {
                bail!(
                    "could not install {}: {error}; the previous file remains at {} because restoring it failed: {restore_error}",
                    destination.display(),
                    backup.display()
                );
            }
            return Err(error).with_context(|| format!("installing {}", destination.display()));
        }
        if let Err(error) = remove_temporary_path(&backup) {
            eprintln!(
                "warning: the new file was installed, but the previous file remains at {}: {error}",
                backup.display()
            );
        }
        return Ok(());
    }

    fs::rename(source, destination).with_context(|| format!("installing {}", destination.display()))
}

fn copy_to_temp(source: &Path, destination: &Path) -> Result<TempFile> {
    let (temp, file) = TempFile::create_near(destination)?;
    drop(file);
    if let Err(error) = fs::copy(source, temp.path()) {
        return Err(cleanup_error(
            &temp,
            format!("copying {} failed: {error}", source.display()),
        ));
    }
    Ok(temp)
}

fn cleanup_error(temp: &TempFile, reason: impl std::fmt::Display) -> anyhow::Error {
    match remove_temporary_path(temp.path()) {
        Ok(()) => anyhow::anyhow!("{reason}. The temporary file was removed."),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            anyhow::anyhow!("{reason}. The temporary file was removed.")
        }
        Err(error) => anyhow::anyhow!(
            "{reason}. Could not remove the temporary file {}: {error}",
            temp.path().display()
        ),
    }
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
    if !destination_exists(path)? || yes {
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
        if io::stdin().read_line(&mut answer)? == 0 {
            bail!("no answer received; overwrite cancelled");
        }
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
    let destination_was_present = destination_exists(path)?;
    let mut response = reqwest::blocking::get(url)
        .map_err(|error| {
            anyhow::anyhow!(
                "download could not connect: {error}. Check your internet connection and try again."
            )
        })?
        .error_for_status()?;
    let (temp, mut file) = TempFile::create_near(path)?;
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
                if show {
                    println!();
                }
                drop(file);
                return Err(cleanup_error(
                    &temp,
                    format!(
                        "Download interrupted. Check your internet connection and try again ({error})"
                    ),
                ));
            }
        };
        if let Err(error) = file.write_all(&buffer[..count]) {
            if show {
                println!();
            }
            drop(file);
            return Err(cleanup_error(
                &temp,
                format!("writing downloaded data failed: {error}"),
            ));
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
        if show {
            println!();
        }
        drop(file);
        return Err(cleanup_error(
            &temp,
            "Download ended before the full file arrived. Check your internet connection and try again",
        ));
    }
    if let Err(error) = file.flush() {
        if show {
            println!();
        }
        drop(file);
        return Err(cleanup_error(
            &temp,
            format!("writing the completed download failed: {error}"),
        ));
    }
    if show {
        let _ = progress(&mut stdout, received, total, speed);
        println!();
    }
    drop(file);
    if !destination_was_present && destination_exists(path)? {
        bail!("destination appeared during download; refusing to overwrite it");
    }
    if let Err(error) = install_file(temp.path(), path) {
        return Err(cleanup_error(
            &temp,
            format!("saving download to {} failed: {error}", path.display()),
        ));
    }
    temp.disarm();
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn run_notification(command: &mut Command, context: &str) -> Result<()> {
    let status = command.status().with_context(|| context.to_owned())?;
    if !status.success() {
        bail!("{context} ({status})");
    }
    Ok(())
}

fn remind(message: &str, at: &str) -> Result<()> {
    let when = DateTime::parse_from_rfc3339(at)
        .context("--at must be an RFC 3339 timestamp with a timezone")?;
    let delay = (when.with_timezone(&Utc) - Utc::now())
        .to_std()
        .context("reminder time must be in the future")?;
    if io::stdout().is_terminal() {
        println!(
            "Reminder set for {}. Keep this process running until it fires.",
            when.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %Z")
        );
    }
    thread::sleep(delay);
    #[cfg(target_os = "linux")]
    run_notification(
        Command::new("notify-send").args(["Reminder", message]),
        "showing notification; install notify-send",
    )?;
    #[cfg(target_os = "macos")]
    run_notification(
        Command::new("osascript").args([
            "-e",
            "on run argv",
            "-e",
            "display notification (item 1 of argv) with title \"Reminder\"",
            "-e",
            "end run",
            message,
        ]),
        "showing notification",
    )?;
    #[cfg(target_os = "windows")]
    {
        let escaped = message.replace('\'', "''");
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $n=New-Object System.Windows.Forms.NotifyIcon; $n.Icon=[System.Drawing.SystemIcons]::Information; $n.BalloonTipTitle='Reminder'; $n.BalloonTipText='{escaped}'; $n.Visible=$true; $n.ShowBalloonTip(10000); Start-Sleep -Seconds 11; $n.Dispose()"
        );
        run_notification(
            Command::new("powershell").args([
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &script,
            ]),
            "showing notification",
        )?;
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
            if same_path(&source, &destination) {
                bail!("source and destination refer to the same file");
            }
            if confirm_overwrite(&destination, yes)? {
                let temp = copy_to_temp(&source, &destination)?;
                if let Err(error) = install_file(temp.path(), &destination) {
                    return Err(cleanup_error(
                        &temp,
                        format!("copying to {} failed: {error}", destination.display()),
                    ));
                }
                temp.disarm();
            }
        }
        FileCommands::Move {
            source,
            destination,
            yes,
        } => {
            if same_path(&source, &destination) {
                bail!("source and destination refer to the same file");
            }
            if confirm_overwrite(&destination, yes)? {
                install_file(&source, &destination).with_context(|| {
                    format!("moving {} to {}", source.display(), destination.display())
                })?;
            }
        }
        FileCommands::Rename {
            path,
            new_path,
            yes,
        } => {
            if same_path(&path, &new_path) {
                bail!("source and destination refer to the same file");
            }
            if confirm_overwrite(&new_path, yes)? {
                install_file(&path, &new_path).with_context(|| {
                    format!("renaming {} to {}", path.display(), new_path.display())
                })?;
            }
        }
        FileCommands::Mkdir { path } => {
            fs::create_dir_all(&path).with_context(|| format!("creating {}", path.display()))?;
        }
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
