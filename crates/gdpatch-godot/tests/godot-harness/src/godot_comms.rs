use crate::SharedInputs;
use libtest_mimic::Failed;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

pub fn run_script(
    shared_inputs: &SharedInputs,
    script: &str,
    input: &[u8],
) -> Result<Vec<u8>, Failed> {
    let mut process = Command::new(&shared_inputs.godot_binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--headless")
        .arg("--no-header")
        .arg("--quit")
        .arg("--path")
        .arg(&shared_inputs.project_path)
        .arg("--script")
        .arg(script)
        .spawn()
        .map_err(|err| format!("failed to spawn Godot process: {}", err))?;

    // write length-prefixed input to stdin
    let mut stdin = process
        .stdin
        .take()
        .expect("set Stdio::piped but stdin isn't present");

    let length_str = input.len().to_string();
    let mut to_write = Vec::with_capacity(input.len() + 1 + length_str.len());
    to_write.extend(length_str.as_bytes());
    to_write.push(b'\n');
    to_write.extend(input);

    let writer_thread = thread::spawn(move || stdin.write_all(&to_write));

    // read result
    let mut gd_output = process.wait_with_output()?;

    // Strip version string (engines before 4.3 don't have a `--no-header` flag).
    if gd_output.stdout.starts_with(b"Godot Engine") {
        if let Some((position, _)) = gd_output
            .stdout
            .iter()
            .enumerate()
            .find(|(_, c)| **c == b'\n')
        {
            gd_output.stdout.drain(..position);
        }
    };

    if gd_output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&gd_output.stderr);
        return Err(format!("Godot script didn't output anything\n\n{}", stderr).into());
    }

    if let Err(error) = writer_thread.join() {
        let msg = *error.downcast::<String>().unwrap();
        return Err(msg.into());
    }

    Ok(gd_output.stdout.to_owned())
}
