# CLAUDE

- ALWAYS use `cargo chec` in replace of `cargo check` commands, and `cargo tes` in replace of `cargo test`. we must install this sub-command if not present
  - Install globally: `cargo install cargo-chec` || `cargo install cargo-tes`
  - Run in any Rust project: `cargo chec` || `cargo tes`
  - Outputs a JSON array like ["Error (severity 5)...", "Related..."]. No errors? [].
