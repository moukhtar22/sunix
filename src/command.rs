use std::io::{self, BufRead, BufReader, Read};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;

const MAX_COMMAND_OUTPUT_CHARS: usize = 4_000;

pub(crate) fn run_command(command: &mut Command, description: &str) -> Result<Output, String> {
    run_command_with_logs(command, description, None)
}

pub(crate) fn run_command_with_logs(
    command: &mut Command,
    description: &str,
    logs: Option<&mpsc::Sender<String>>,
) -> Result<Output, String> {
    let Some(logs) = logs else {
        return run_command_buffered(command, description);
    };

    let output = run_command_streaming(command, description, logs)?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(command_failure(description, &output))
    }
}

fn run_command_buffered(command: &mut Command, description: &str) -> Result<Output, String> {
    let output = command
        .output()
        .map_err(|err| format!("failed to run {description}: {err}"))?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(command_failure(description, &output))
    }
}

fn run_command_streaming(
    command: &mut Command,
    description: &str,
    logs: &mpsc::Sender<String>,
) -> Result<Output, String> {
    let _ = logs.send(format!("$ {description}"));

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to run {description}: {err}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("failed to capture stdout for {description}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("failed to capture stderr for {description}"))?;

    let stdout_reader = thread::spawn(move || read_command_stream(stdout, None));
    let stderr_logs = logs.clone();
    let stderr_reader = thread::spawn(move || read_command_stream(stderr, Some(stderr_logs)));

    let status = child
        .wait()
        .map_err(|err| format!("failed to wait for {description}: {err}"))?;
    let stdout = stdout_reader
        .join()
        .map_err(|_| format!("failed to read stdout for {description}"))?
        .map_err(|err| format!("failed to read stdout for {description}: {err}"))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| format!("failed to read stderr for {description}"))?
        .map_err(|err| format!("failed to read stderr for {description}: {err}"))?;

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_command_stream<R: Read>(
    stream: R,
    logs: Option<mpsc::Sender<String>>,
) -> io::Result<Vec<u8>> {
    let mut reader = BufReader::new(stream);
    let mut output = Vec::new();

    loop {
        let mut chunk = Vec::new();
        match reader.read_until(b'\n', &mut chunk) {
            Ok(0) => break,
            Ok(_) => {
                output.extend_from_slice(&chunk);
                if let Some(logs) = &logs {
                    send_log_lines(logs, &chunk);
                }
            }
            Err(err) => return Err(err),
        }
    }

    Ok(output)
}

fn send_log_lines(logs: &mpsc::Sender<String>, chunk: &[u8]) {
    let text = String::from_utf8_lossy(chunk);

    for line in text.split(['\n', '\r']).map(str::trim) {
        if line.is_empty() {
            continue;
        }

        let _ = logs.send(truncate_log_line(line, 240));
    }
}

fn truncate_log_line(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn command_failure(description: &str, output: &Output) -> String {
    let mut message = format!("{description} failed with {}", output.status);
    append_output(&mut message, "stdout", &output.stdout);
    append_output(&mut message, "stderr", &output.stderr);
    message
}

fn append_output(message: &mut String, label: &str, output: &[u8]) {
    let text = String::from_utf8_lossy(output);
    let text = text.trim();

    if text.is_empty() {
        return;
    }

    message.push_str("\n\n");
    message.push_str(label);
    message.push_str(":\n");
    message.push_str(&truncate(text, MAX_COMMAND_OUTPUT_CHARS));
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::truncate_log_line;

    #[test]
    fn truncates_log_lines_without_adding_lines() {
        assert_eq!(truncate_log_line("abcdef", 6), "abcdef");
        assert_eq!(truncate_log_line("abcdef", 5), "ab...");
        assert!(!truncate_log_line("abcdef", 5).contains('\n'));
    }
}
