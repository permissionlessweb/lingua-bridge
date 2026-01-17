- ALWAYS use `cargo chec` in replace of `cargo check` commands. we must install this sub-command if not present
  - Install globally: `cargo install cargo-chec`
  - Run in any Rust project: `cargo chec`
  - Outputs a JSON array like ["Error (severity 5)...", "Related..."]. No errors? [].

