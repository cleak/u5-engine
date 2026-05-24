//! Disk I/O retry wrappers.

use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskIoHandlerPhase {
    ReadPrompt,
    WritePrompt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskIoRetryEvent {
    pub phase: DiskIoHandlerPhase,
    pub attempt: usize,
    pub file_name: String,
}

impl DiskIoRetryEvent {
    pub fn prompt_message(&self) -> String {
        disk_io_retry_prompt_message(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskRetryPolicy {
    pub max_retries: Option<usize>,
}

impl DiskRetryPolicy {
    pub const fn single_directory() -> Self {
        Self {
            max_retries: Some(0),
        }
    }

    pub const fn unbounded() -> Self {
        Self { max_retries: None }
    }
}

pub fn read_disk_file(path: &Path) -> io::Result<Vec<u8>> {
    read_disk_file_with_policy(path, DiskRetryPolicy::single_directory())
}

pub fn read_optional_disk_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match read_disk_file(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn read_disk_file_with_policy(path: &Path, policy: DiskRetryPolicy) -> io::Result<Vec<u8>> {
    let file_name = disk_file_name(path);
    read_with_retry(
        &file_name,
        policy,
        || {
            fs::read(path)
                .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))
        },
        |_| Ok(()),
    )
}

pub fn write_disk_file(path: &Path, bytes: impl AsRef<[u8]>) -> io::Result<usize> {
    write_disk_file_with_policy(path, bytes.as_ref(), DiskRetryPolicy::single_directory())
}

pub fn write_disk_file_with_policy(
    path: &Path,
    bytes: &[u8],
    policy: DiskRetryPolicy,
) -> io::Result<usize> {
    let file_name = disk_file_name(path);
    write_with_retry(
        &file_name,
        policy,
        || {
            fs::write(path, bytes)
                .map(|_| bytes.len())
                .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))
        },
        |_| Ok(()),
        |_| {},
    )
}

pub fn disk_io_retry_prompt_message(event: &DiskIoRetryEvent) -> String {
    let verb = match event.phase {
        DiskIoHandlerPhase::ReadPrompt => "read",
        DiskIoHandlerPhase::WritePrompt => "write",
    };
    format!(
        "Disk {verb} retry {} for {}. Press any key after the disk is ready.",
        event.attempt, event.file_name
    )
}

pub fn disk_io_error_message(
    phase: DiskIoHandlerPhase,
    file_name: &str,
    err: &io::Error,
) -> String {
    let verb = match phase {
        DiskIoHandlerPhase::ReadPrompt => "read",
        DiskIoHandlerPhase::WritePrompt => "write",
    };
    let action = match phase {
        DiskIoHandlerPhase::ReadPrompt => "Check the mounted game/save directory and try again.",
        DiskIoHandlerPhase::WritePrompt => {
            "Check that the save directory is writable and try again."
        }
    };
    format!("Disk {verb} failed for {file_name}: {err}. {action}")
}

pub fn read_with_retry<R, P>(
    file_name: &str,
    policy: DiskRetryPolicy,
    mut read_once: R,
    mut prompt: P,
) -> io::Result<Vec<u8>>
where
    R: FnMut() -> io::Result<Vec<u8>>,
    P: FnMut(DiskIoRetryEvent) -> io::Result<()>,
{
    let mut retries = 0;
    loop {
        match read_once() {
            Ok(bytes) if !bytes.is_empty() => return Ok(bytes),
            Ok(_) => {
                if !retry_available(policy, retries) {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("{file_name}: zero-byte disk read"),
                    ));
                }
            }
            Err(err) => {
                if !retry_available(policy, retries) {
                    return Err(err);
                }
            }
        }
        retries += 1;
        prompt(DiskIoRetryEvent {
            phase: DiskIoHandlerPhase::ReadPrompt,
            attempt: retries,
            file_name: file_name.to_string(),
        })?;
    }
}

pub fn write_with_retry<W, P, H>(
    file_name: &str,
    policy: DiskRetryPolicy,
    mut write_once: W,
    mut prompt: P,
    mut set_handler_phase: H,
) -> io::Result<usize>
where
    W: FnMut() -> io::Result<usize>,
    P: FnMut(DiskIoRetryEvent) -> io::Result<()>,
    H: FnMut(DiskIoHandlerPhase),
{
    set_handler_phase(DiskIoHandlerPhase::WritePrompt);
    let result = write_with_retry_inner(file_name, policy, &mut write_once, &mut prompt);
    set_handler_phase(DiskIoHandlerPhase::ReadPrompt);
    result
}

fn write_with_retry_inner<W, P>(
    file_name: &str,
    policy: DiskRetryPolicy,
    write_once: &mut W,
    prompt: &mut P,
) -> io::Result<usize>
where
    W: FnMut() -> io::Result<usize>,
    P: FnMut(DiskIoRetryEvent) -> io::Result<()>,
{
    let mut retries = 0;
    loop {
        match write_once() {
            Ok(count) if count != 0 => return Ok(count),
            Ok(_) => {
                if !retry_available(policy, retries) {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        format!("{file_name}: zero-byte disk write"),
                    ));
                }
            }
            Err(err) => {
                if !retry_available(policy, retries) {
                    return Err(err);
                }
            }
        }
        retries += 1;
        prompt(DiskIoRetryEvent {
            phase: DiskIoHandlerPhase::WritePrompt,
            attempt: retries,
            file_name: file_name.to_string(),
        })?;
    }
}

const fn retry_available(policy: DiskRetryPolicy, retries: usize) -> bool {
    match policy.max_retries {
        Some(max_retries) => retries < max_retries,
        None => true,
    }
}

fn disk_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unnamed>")
        .to_string()
}
