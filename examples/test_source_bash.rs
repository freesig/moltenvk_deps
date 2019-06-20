use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

fn main() {
    let before = env::var("TEST_SOURCE");
    let result = Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()
        .and_then(|mut output| {
            output
                .stdin
                .as_mut()
                .and_then(|stdin| {
                    stdin
                        .write_all(b"source ~/.bash_profile\n")
                        .ok()
                        .and_then(|_| stdin.write_all(b"source ~/.bash_login\n").ok())
                        .and_then(|_| stdin.write_all(b"source ~/.profile\n").ok())
                        .and_then(|_| stdin.write_all(b"echo $PATH").ok())
                })
                .and_then(|_| {
                    output.wait_with_output().ok().map(|output| {
                        let path = String::from_utf8_lossy(&output.stdout);
                        env::split_paths(&path.trim()).collect::<Vec<_>>()
                    })
                })
        });
    dbg!(result);
    let after = env::var("TEST_SOURCE");
    dbg!(&before);
    dbg!(&after);
}
