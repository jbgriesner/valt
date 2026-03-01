mod tui;

use std::path::PathBuf;

fn main() {
    let vault_path = default_vault_path();
    if let Err(e) = tui::run(vault_path) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn default_vault_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("valt");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("vault.svlt")
}
