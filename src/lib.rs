use chrono::{DateTime, NaiveDateTime};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{QueueableCommand, cursor};
use pyo3::exceptions::{PyConnectionError, PyFileExistsError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn io_error(error: std::io::Error) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyOSError, _>(error.to_string())
}

fn checked_destination(path: PathBuf, overwrite: bool) -> PyResult<PathBuf> {
    if path.exists() && !overwrite {
        return Err(PyFileExistsError::new_err(format!(
            "destination already exists: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn temporary_download(path: &PathBuf) -> PyResult<(PathBuf, fs::File)> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| PyValueError::new_err("download destination must name a file"))?;
    for attempt in 0..100u32 {
        let mut temporary_name = name.to_os_string();
        temporary_name.push(format!(".daily-{}-{attempt}.part", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Err(PyValueError::new_err(
        "could not create a temporary download file",
    ))
}

fn confirm_overwrite(path: &PathBuf, overwrite: Option<bool>) -> PyResult<bool> {
    if !path.exists() {
        return Ok(overwrite.unwrap_or(false));
    }
    if let Some(overwrite) = overwrite {
        return Ok(overwrite);
    }

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    if !stdout.is_terminal() || !stdin.is_terminal() {
        return Err(PyFileExistsError::new_err(format!(
            "destination already exists: {} (pass overwrite=True to replace it)",
            path.display()
        )));
    }
    stdout
        .queue(SetForegroundColor(Color::Yellow))
        .and_then(|out| out.queue(Print("File already exists: ")))
        .and_then(|out| out.queue(SetForegroundColor(Color::Cyan)))
        .and_then(|out| out.queue(Print(path.display().to_string())))
        .and_then(|out| out.queue(SetForegroundColor(Color::Magenta)))
        .and_then(|out| out.queue(Print("\nOverwrite it? (y/n) > ")))
        .and_then(|out| out.queue(ResetColor))
        .and_then(|out| out.flush())
        .map_err(|error| {
            PyValueError::new_err(format!("could not display overwrite prompt: {error}"))
        })?;

    loop {
        let mut answer = String::new();
        stdin.read_line(&mut answer).map_err(|error| {
            PyValueError::new_err(format!("could not read overwrite choice: {error}"))
        })?;
        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => {
                stdout
                    .queue(SetForegroundColor(Color::DarkGrey))
                    .and_then(|out| out.queue(Print("Download cancelled.\n")))
                    .and_then(|out| out.queue(ResetColor))
                    .and_then(|out| out.flush())
                    .map_err(|error| {
                        PyValueError::new_err(format!("could not display cancellation: {error}"))
                    })?;
                return Ok(false);
            }
            _ => {
                stdout
                    .queue(SetForegroundColor(Color::Yellow))
                    .and_then(|out| out.queue(Print("Please answer y or n > ")))
                    .and_then(|out| out.queue(ResetColor))
                    .and_then(|out| out.flush())
                    .map_err(|error| {
                        PyValueError::new_err(format!(
                            "could not display overwrite prompt: {error}"
                        ))
                    })?;
            }
        }
    }
}

fn render_progress(
    stdout: &mut io::Stdout,
    received: u64,
    total: Option<u64>,
    speed_mb_per_second: f64,
) -> io::Result<()> {
    const WIDTH: usize = 30;
    const MB: f64 = 1_000_000.0;
    stdout.queue(cursor::MoveToColumn(0))?;
    if let Some(total) = total.filter(|total| *total > 0) {
        let percent = ((received.saturating_mul(100) / total).min(100)) as usize;
        let filled = percent * WIDTH / 100;
        stdout
            .queue(Print("["))?
            .queue(SetForegroundColor(Color::Cyan))?
            .queue(Print("#".repeat(filled)))?
            .queue(SetForegroundColor(Color::DarkGrey))?
            .queue(Print("-".repeat(WIDTH - filled)))?
            .queue(ResetColor)?
            .queue(Print(format!(
                "] {percent:3}%  {:.2} MB / {:.2} MB  {:.2} MB/s",
                received as f64 / MB,
                total as f64 / MB,
                speed_mb_per_second
            )))?;
    } else {
        stdout
            .queue(SetForegroundColor(Color::Yellow))?
            .queue(Print(format!(
                "Downloaded {:.2} MB at {:.2} MB/s (size unknown)",
                received as f64 / MB,
                speed_mb_per_second
            )))?
            .queue(ResetColor)?;
    }
    stdout.queue(Clear(ClearType::UntilNewLine))?.flush()
}

fn interrupted_download(stdout: &mut io::Stdout, show_progress: bool, error: &str) -> PyErr {
    let message = format!(
        "download interrupted: {error}. The partial file was removed. Check your internet connection and try again."
    );
    if show_progress {
        let _ = stdout
            .queue(Print("\n"))
            .and_then(|out| out.queue(SetForegroundColor(Color::Red)))
            .and_then(|out| out.queue(Print(format!("{message}\n"))))
            .and_then(|out| out.queue(ResetColor))
            .and_then(|out| out.flush());
    }
    PyConnectionError::new_err(message)
}

#[pyfunction]
#[pyo3(signature = (url, path, overwrite=None))]
fn download_url(url: &str, path: PathBuf, overwrite: Option<bool>) -> PyResult<Option<PathBuf>> {
    let overwrite = confirm_overwrite(&path, overwrite)?;
    if path.exists() && !overwrite {
        return Ok(None);
    }
    let path = checked_destination(path, overwrite)?;
    let show_progress = io::stdout().is_terminal();
    let mut stdout = io::stdout();
    let mut response = reqwest::blocking::get(url)
        .map_err(|error| interrupted_download(&mut stdout, show_progress, &error.to_string()))?
        .error_for_status()
        .map_err(|error| PyValueError::new_err(format!("download failed: {error}")))?;
    let (temporary_path, mut file) = temporary_download(&path)?;
    let total = response.content_length();
    let mut received = 0u64;
    let mut last_update = Instant::now() - Duration::from_secs(1);
    let mut speed_sample_time = Instant::now();
    let mut speed_sample_received = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = match response.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(error) => {
                drop(file);
                let _ = fs::remove_file(&temporary_path);
                return Err(interrupted_download(
                    &mut stdout,
                    show_progress,
                    &error.to_string(),
                ));
            }
        };
        if let Err(error) = file.write_all(&buffer[..count]) {
            drop(file);
            if show_progress {
                let _ = stdout.queue(Print("\n")).and_then(|out| out.flush());
            }
            let _ = fs::remove_file(&temporary_path);
            return Err(io_error(error));
        }
        received += count as u64;
        if show_progress && last_update.elapsed() >= Duration::from_millis(100) {
            let elapsed = speed_sample_time.elapsed().as_secs_f64().max(0.001);
            let speed_mb_per_second =
                (received - speed_sample_received) as f64 / elapsed / 1_000_000.0;
            let _ = render_progress(&mut stdout, received, total, speed_mb_per_second);
            last_update = Instant::now();
            speed_sample_time = last_update;
            speed_sample_received = received;
        }
    }
    if total.is_some_and(|expected| received < expected) {
        drop(file);
        let _ = fs::remove_file(&temporary_path);
        return Err(interrupted_download(
            &mut stdout,
            show_progress,
            "the server connection closed before the full file arrived",
        ));
    }
    if show_progress {
        let elapsed = speed_sample_time.elapsed().as_secs_f64().max(0.001);
        let speed_mb_per_second = (received - speed_sample_received) as f64 / elapsed / 1_000_000.0;
        let _ = render_progress(&mut stdout, received, total, speed_mb_per_second);
        let _ = stdout.queue(Print("\n")).and_then(|out| out.flush());
    }
    if let Err(error) = file.flush() {
        drop(file);
        let _ = fs::remove_file(&temporary_path);
        return Err(io_error(error));
    }
    drop(file);
    if overwrite {
        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(&path).map_err(io_error)?;
        }
        fs::rename(&temporary_path, &path).map_err(|error| {
            let _ = fs::remove_file(&temporary_path);
            io_error(error)
        })?;
    } else {
        fs::hard_link(&temporary_path, &path).map_err(|error| {
            let _ = fs::remove_file(&temporary_path);
            io_error(error)
        })?;
        fs::remove_file(&temporary_path).map_err(io_error)?;
    }
    Ok(Some(path))
}

#[pyfunction]
#[pyo3(signature = (source, destination, overwrite=false))]
fn copy_file(source: PathBuf, destination: PathBuf, overwrite: bool) -> PyResult<PathBuf> {
    let destination_path = checked_destination(destination, overwrite)?;
    if overwrite {
        fs::copy(source, &destination_path).map_err(io_error)?;
    } else {
        let mut input = fs::File::open(source).map_err(io_error)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination_path)
            .map_err(io_error)?;
        if let Err(error) = std::io::copy(&mut input, &mut output) {
            drop(output);
            let _ = fs::remove_file(&destination_path);
            return Err(io_error(error));
        }
    }
    Ok(destination_path)
}

#[pyfunction]
#[pyo3(signature = (source, destination, overwrite=false))]
fn move_file(source: PathBuf, destination: PathBuf, overwrite: bool) -> PyResult<PathBuf> {
    if source == destination {
        return Ok(destination);
    }
    let destination_path = checked_destination(destination, overwrite)?;
    if overwrite && destination_path.exists() {
        fs::remove_file(&destination_path).map_err(io_error)?;
    }
    fs::rename(source, &destination_path).map_err(io_error)?;
    Ok(destination_path)
}

#[pyfunction]
#[pyo3(signature = (path, new_path, overwrite=false))]
fn rename_file(path: PathBuf, new_path: PathBuf, overwrite: bool) -> PyResult<PathBuf> {
    if path == new_path {
        return Ok(new_path);
    }
    let new_path = checked_destination(new_path, overwrite)?;
    if overwrite && new_path.exists() {
        fs::remove_file(&new_path).map_err(io_error)?;
    }
    fs::rename(path, &new_path).map_err(io_error)?;
    Ok(new_path)
}

#[pyfunction]
#[pyo3(signature = (path, parents=true, exist_ok=true))]
fn create_directory(path: PathBuf, parents: bool, exist_ok: bool) -> PyResult<PathBuf> {
    let path = path.as_path();
    let result = if parents {
        fs::create_dir_all(path)
    } else {
        fs::create_dir(path)
    };
    match result {
        Ok(()) => Ok(path.to_path_buf()),
        Err(_error) if exist_ok && path.is_dir() => Ok(path.to_path_buf()),
        Err(error) => Err(io_error(error)),
    }
}

#[pyfunction]
fn format_time(value: &str, format: &str) -> PyResult<String> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|error| {
        PyValueError::new_err(format!("expected an RFC 3339 datetime: {error}"))
    })?;
    Ok(parsed.format(format).to_string())
}

#[pyfunction]
fn parse_time(value: &str, format: &str) -> PyResult<String> {
    let parsed = NaiveDateTime::parse_from_str(value, format)
        .map_err(|error| PyValueError::new_err(format!("could not parse datetime: {error}")))?;
    Ok(parsed.and_utc().to_rfc3339())
}

#[pyfunction]
fn format_duration(seconds: f64) -> PyResult<String> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(PyValueError::new_err(
            "seconds must be a finite non-negative number",
        ));
    }
    let total = seconds.floor() as u64;
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;
    let secs = total % 60;
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
    Ok(parts.join(" "))
}

#[pymodule]
fn _daily(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(download_url, module)?)?;
    module.add_function(wrap_pyfunction!(copy_file, module)?)?;
    module.add_function(wrap_pyfunction!(move_file, module)?)?;
    module.add_function(wrap_pyfunction!(rename_file, module)?)?;
    module.add_function(wrap_pyfunction!(create_directory, module)?)?;
    module.add_function(wrap_pyfunction!(format_time, module)?)?;
    module.add_function(wrap_pyfunction!(parse_time, module)?)?;
    module.add_function(wrap_pyfunction!(format_duration, module)?)?;
    Ok(())
}
