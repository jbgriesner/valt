use std::io::{self, BufRead};
use std::path::PathBuf;

use crate::core::{generate, GeneratorConfig, Secret, VaultManager};
use clap::{Parser, Subcommand};
use serdevault::VaultFile;

#[derive(Parser)]
#[command(
    name = "valt",
    about = "Keyboard-driven terminal password manager",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// List secrets, with an optional fuzzy filter
    List {
        /// Fuzzy search query (e.g. "github")
        query: Option<String>,
    },

    /// Print the password of the best-matching secret to stdout
    ///
    /// Matched name and username are printed to stderr so that only the
    /// password reaches stdout, making the command scriptable:
    ///   export TOKEN=$(valt get myapi)
    Get {
        /// Name to search for (fuzzy)
        name: String,
    },

    /// Add a new secret
    Add {
        /// Secret name
        name: String,

        /// Username / login
        #[arg(long, short)]
        username: Option<String>,

        /// URL associated with this secret
        #[arg(long)]
        url: Option<String>,

        /// Comma-separated tags (e.g. "work,ssh")
        #[arg(long, short)]
        tags: Option<String>,

        /// Generate a random password instead of prompting
        #[arg(long, short)]
        generate: bool,
    },

    /// Delete the best-matching secret
    Rm {
        /// Name to search for (fuzzy)
        name: String,

        /// Skip the confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },
}

pub fn run_command(
    command: Command,
    vault_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Command::List { query } => cmd_list(vault_path, query.as_deref()),
        Command::Get { name } => cmd_get(vault_path, &name),
        Command::Add {
            name,
            username,
            url,
            tags,
            generate: gen,
        } => cmd_add(
            vault_path,
            &name,
            username.as_deref(),
            url.as_deref(),
            tags.as_deref(),
            gen,
        ),
        Command::Rm { name, yes } => cmd_rm(vault_path, &name, yes),
    }
}

fn prompt_vault_password() -> Result<String, Box<dyn std::error::Error>> {
    Ok(rpassword::prompt_password("Vault password: ")?)
}

/// Open an existing vault — fails with a helpful message if the file is absent.
fn open_vault(vault_path: &PathBuf) -> Result<VaultManager, Box<dyn std::error::Error>> {
    if !vault_path.exists() {
        return Err("Vault not found. Add your first secret with `valt add` \
             or launch `valt` to open the TUI."
            .into());
    }
    let password = prompt_vault_password()?;
    let vf = VaultFile::open(vault_path, &password);
    VaultManager::open(vf)
        .map(|m| m.with_backup_path(vault_path.clone()))
        .map_err(|_| "Wrong password or corrupted vault.".into())
}

/// Open existing vault or create a new one (used by `add`).
fn open_or_create_vault(vault_path: &PathBuf) -> Result<VaultManager, Box<dyn std::error::Error>> {
    let password = prompt_vault_password()?;
    let vf = VaultFile::open(vault_path, &password);
    VaultManager::open_or_create(vf)
        .map(|m| m.with_backup_path(vault_path.clone()))
        .map_err(|e| format!("Failed to open vault: {e}").into())
}

fn cmd_list(vault_path: &PathBuf, query: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let vault = open_vault(vault_path)?;
    let results = vault.search(query.unwrap_or(""));

    if results.is_empty() {
        eprintln!("No secrets found.");
        return Ok(());
    }

    for s in &results {
        let username = s.username.as_deref().unwrap_or("");
        let url = s.url.as_deref().unwrap_or("");
        println!("  {:<30}  {:<24}  {}", s.name, username, url);
    }

    Ok(())
}

fn cmd_get(vault_path: &PathBuf, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let vault = open_vault(vault_path)?;
    let results = vault.search(name);

    match results.first() {
        Some(s) => {
            // Metadata to stderr — only the password reaches stdout.
            let username = s.username.as_deref().unwrap_or("");
            if username.is_empty() {
                eprintln!("Matched: {}", s.name);
            } else {
                eprintln!("Matched: {} ({})", s.name, username);
            }
            println!("{}", s.password);
            Ok(())
        }
        None => Err(format!("No secret matching '{name}'.").into()),
    }
}

fn cmd_add(
    vault_path: &PathBuf,
    name: &str,
    username: Option<&str>,
    url: Option<&str>,
    tags: Option<&str>,
    gen: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut vault = open_or_create_vault(vault_path)?;

    // Warn if a secret with the same name already exists.
    if vault.search(name).iter().any(|s| s.name == name) {
        eprintln!("Warning: a secret named '{name}' already exists.");
    }

    let password = if gen {
        let pwd = generate(&GeneratorConfig::default())?;
        eprintln!("Generated: {pwd}");
        pwd
    } else {
        let p1 = rpassword::prompt_password("Password: ")?;
        let p2 = rpassword::prompt_password("Confirm:  ")?;
        if p1 != p2 {
            return Err("Passwords do not match.".into());
        }
        p1
    };

    let mut secret = Secret::new(name, &password);
    if let Some(u) = username {
        secret.username = Some(u.to_string());
    }
    if let Some(u) = url {
        secret.url = Some(u.to_string());
    }
    if let Some(t) = tags {
        secret.tags = t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    vault.add(secret)?;
    eprintln!("Secret '{name}' saved.");
    Ok(())
}

fn cmd_rm(vault_path: &PathBuf, name: &str, yes: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut vault = open_vault(vault_path)?;
    let results = vault.search(name);

    let secret = results
        .first()
        .ok_or_else(|| format!("No secret matching '{name}'."))?;

    let id = secret.id;
    let secret_name = secret.name.clone();

    if !yes {
        eprint!("Delete '{secret_name}'? [y/N] ");
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if line.trim().to_lowercase() != "y" {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    vault.delete(id)?;
    eprintln!("Secret '{secret_name}' deleted.");
    Ok(())
}
