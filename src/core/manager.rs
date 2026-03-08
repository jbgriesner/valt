use std::path::{Path, PathBuf};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serdevault::VaultFile;
use uuid::Uuid;

use super::{error::CoreError, secret::Secret, vault_data::VaultData};

/// High-level interface to the encrypted vault.
pub struct VaultManager {
    vault: VaultFile,
    data: VaultData,
    /// When set, `save()` copies the current file to `<path>.bak` before each write.
    path: Option<PathBuf>,
}

impl VaultManager {
    /// Open an existing vault. Returns `Vault(DecryptionFailed)` if the
    /// password is wrong or the file is corrupted.
    pub fn open(vault: VaultFile) -> Result<Self, CoreError> {
        let data = vault.load::<VaultData>()?;
        Ok(Self {
            vault,
            data,
            path: None,
        })
    }

    /// Create a new, empty vault. Writes the initial (empty) `VaultData` to
    /// disk immediately so the file exists and is valid from the start.
    pub fn create(vault: VaultFile) -> Result<Self, CoreError> {
        let data = VaultData::default();
        vault.save(&data)?;
        Ok(Self {
            vault,
            data,
            path: None,
        })
    }

    /// Open the vault if its file exists, or create a new one otherwise.
    /// This is the typical entry point for the application.
    pub fn open_or_create(vault: VaultFile) -> Result<Self, CoreError> {
        if vault.exists() {
            Self::open(vault)
        } else {
            Self::create(vault)
        }
    }

    /// Enable automatic backup: before every `save()`, the current vault file
    /// is copied to `<path>.bak`. Call this right after construction.
    pub fn with_backup_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// All secrets, in insertion order.
    pub fn list(&self) -> &[Secret] {
        &self.data.secrets
    }

    /// Look up a secret by its UUID.
    pub fn get(&self, id: Uuid) -> Option<&Secret> {
        self.data.secrets.iter().find(|s| s.id == id)
    }

    /// Fuzzy search over `name`, `username`, `url`, and `tags`.
    pub fn search(&self, query: &str) -> Vec<&Secret> {
        if query.is_empty() {
            return self.data.secrets.iter().collect();
        }

        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, &Secret)> = self
            .data
            .secrets
            .iter()
            .filter_map(|s| {
                let score = Self::match_score(&matcher, s, query);
                score.map(|sc| (sc, s))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, s)| s).collect()
    }

    /// Add a new secret and persist the vault.
    pub fn add(&mut self, secret: Secret) -> Result<(), CoreError> {
        self.data.secrets.push(secret);
        self.save()
    }

    /// Replace the secret with the given `id` and persist the vault.
    /// The `updated.id` field is ignored — the original `id` is preserved.
    pub fn update(&mut self, id: Uuid, mut updated: Secret) -> Result<(), CoreError> {
        let entry = self
            .data
            .secrets
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or(CoreError::NotFound(id))?;

        updated.id = id;
        updated.created_at = entry.created_at;
        updated.touch();
        *entry = updated;

        self.save()
    }

    /// Remove the secret with the given `id` and persist the vault.
    pub fn delete(&mut self, id: Uuid) -> Result<(), CoreError> {
        let before = self.data.secrets.len();
        self.data.secrets.retain(|s| s.id != id);

        if self.data.secrets.len() == before {
            return Err(CoreError::NotFound(id));
        }

        self.save()
    }

    /// Persist the current in-memory state to disk.
    ///
    /// If `with_backup_path` was called, copies the existing vault file to
    /// `<path>.bak` before writing the new version. The backup reflects the
    /// state just before the current save, so a single rollback is always
    /// possible. Fails hard if the backup copy itself fails — a copy error
    /// usually indicates a filesystem problem that would compromise the write
    /// too.
    pub fn save(&self) -> Result<(), CoreError> {
        if let Some(ref path) = self.path {
            if path.exists() {
                let bak = bak_path(path);
                std::fs::copy(path, &bak).map_err(CoreError::Backup)?;
            }
        }
        self.vault.save(&self.data).map_err(CoreError::Vault)
    }

    /// Compute the best fuzzy match score for a secret against a query string.
    /// Returns `None` if the secret does not match at all.
    fn match_score(matcher: &SkimMatcherV2, secret: &Secret, query: &str) -> Option<i64> {
        let candidates = [
            Some(secret.name.as_str()),
            secret.username.as_deref(),
            secret.url.as_deref(),
        ];

        let best_field = candidates
            .iter()
            .flatten()
            .filter_map(|text| matcher.fuzzy_match(text, query))
            .max();

        let best_tag = secret
            .tags
            .iter()
            .filter_map(|tag| matcher.fuzzy_match(tag, query))
            .max();

        match (best_field, best_tag) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0).max(b.unwrap_or(0))),
        }
    }
}

/// Returns the backup path for a vault file: `<path>.bak`
/// e.g. `/home/user/.local/share/valt/vault.svlt` → `vault.svlt.bak`
fn bak_path(path: &Path) -> PathBuf {
    let mut bak = path.to_path_buf();
    let name = path
        .file_name()
        .map(|n| format!("{}.bak", n.to_string_lossy()))
        .unwrap_or_else(|| "vault.svlt.bak".to_string());
    bak.set_file_name(name);
    bak
}

#[cfg(test)]
mod tests {
    use super::*;
    use serdevault::VaultFile;
    use tempfile::tempdir;

    // Fast Argon2 params for tests
    const M: u32 = 8;
    const T: u32 = 1;
    const P: u32 = 1;

    fn test_vault(dir: &tempfile::TempDir) -> VaultFile {
        VaultFile::open(dir.path().join("vault.svlt"), "test_password").with_params(M, T, P)
    }

    fn make_secret(name: &str, password: &str) -> Secret {
        Secret::new(name, password)
    }

    // 1. open_or_create on a new path → creates empty vault
    #[test]
    fn test_create_empty() {
        let dir = tempdir().unwrap();
        let mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        assert_eq!(mgr.list().len(), 0);
    }

    // 2. add → list returns the secret
    #[test]
    fn test_add_and_list() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        mgr.add(make_secret("GitHub", "s3cr3t")).unwrap();
        assert_eq!(mgr.list().len(), 1);
        assert_eq!(mgr.list()[0].name, "GitHub");
    }

    // 3. add persists: a fresh manager loaded from same file sees the secret
    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();

        {
            let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
            mgr.add(make_secret("GitHub", "s3cr3t")).unwrap();
        }

        let mgr2 = VaultManager::open(test_vault(&dir)).unwrap();
        assert_eq!(mgr2.list().len(), 1);
        assert_eq!(mgr2.list()[0].name, "GitHub");
    }

    // 4. get by id → Some(secret)
    #[test]
    fn test_get_existing() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let s = make_secret("GitHub", "s3cr3t");
        let id = s.id;
        mgr.add(s).unwrap();

        let found = mgr.get(id).unwrap();
        assert_eq!(found.name, "GitHub");
    }

    // 5. get non-existing id → None
    #[test]
    fn test_get_missing() {
        let dir = tempdir().unwrap();
        let mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        assert!(mgr.get(Uuid::new_v4()).is_none());
    }

    // 6. update → fields change, id and created_at preserved
    #[test]
    fn test_update() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let original = make_secret("GitHub", "old");
        let id = original.id;
        let created = original.created_at;
        mgr.add(original).unwrap();

        let updated = Secret::new("GitHub perso", "new_password");
        mgr.update(id, updated).unwrap();

        let found = mgr.get(id).unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.name, "GitHub perso");
        assert_eq!(found.password, "new_password");
        assert_eq!(found.created_at, created); // created_at unchanged
        assert!(found.updated_at > created); // updated_at bumped
    }

    // 7. update non-existing id → NotFound
    #[test]
    fn test_update_missing() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let err = mgr
            .update(Uuid::new_v4(), make_secret("X", "y"))
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound(_)));
    }

    // 8. delete → secret is gone
    #[test]
    fn test_delete() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let s = make_secret("GitHub", "s3cr3t");
        let id = s.id;
        mgr.add(s).unwrap();
        mgr.delete(id).unwrap();
        assert_eq!(mgr.list().len(), 0);
        assert!(mgr.get(id).is_none());
    }

    // 9. delete non-existing id → NotFound
    #[test]
    fn test_delete_missing() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let err = mgr.delete(Uuid::new_v4()).unwrap_err();
        assert!(matches!(err, CoreError::NotFound(_)));
    }

    // 10. search empty query → all secrets
    #[test]
    fn test_search_empty_query() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        mgr.add(make_secret("GitHub", "a")).unwrap();
        mgr.add(make_secret("GitLab", "b")).unwrap();
        assert_eq!(mgr.search("").len(), 2);
    }

    // 11. search fuzzy on name
    #[test]
    fn test_search_fuzzy_name() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        mgr.add(make_secret("GitHub perso", "a")).unwrap();
        mgr.add(make_secret("AWS console", "b")).unwrap();

        let results = mgr.search("git");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "GitHub perso");
    }

    // 12. search fuzzy on url
    #[test]
    fn test_search_fuzzy_url() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let mut s = make_secret("Work email", "a");
        s.url = Some("https://mail.company.com".to_string());
        mgr.add(s).unwrap();
        mgr.add(make_secret("GitHub", "b")).unwrap();

        let results = mgr.search("mail");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Work email");
    }

    // 13. search fuzzy on tags
    #[test]
    fn test_search_fuzzy_tags() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        let mut s = make_secret("Server root", "a");
        s.tags = vec!["linux".to_string(), "ops".to_string()];
        mgr.add(s).unwrap();
        mgr.add(make_secret("GitHub", "b")).unwrap();

        let results = mgr.search("ops");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Server root");
    }

    // 14. search no match → empty vec
    #[test]
    fn test_search_no_match() {
        let dir = tempdir().unwrap();
        let mut mgr = VaultManager::open_or_create(test_vault(&dir)).unwrap();
        mgr.add(make_secret("GitHub", "a")).unwrap();

        let results = mgr.search("zzzzzzzzzzz");
        assert!(results.is_empty());
    }

    // 15. wrong password → DecryptionFailed propagated as CoreError::Vault
    #[test]
    fn test_wrong_password() {
        let dir = tempdir().unwrap();
        VaultManager::open_or_create(test_vault(&dir)).unwrap();

        let wrong = VaultFile::open(dir.path().join("vault.svlt"), "wrong").with_params(M, T, P);
        let result = VaultManager::open(wrong);
        assert!(matches!(result, Err(CoreError::Vault(_))));
    }
}
