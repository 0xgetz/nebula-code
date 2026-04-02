//! Persistence layer for the Nebula Marketplace skill storage.
//!
//! This module provides a trait-based storage abstraction that supports multiple
//! backends. The default implementation (`FileSkillStorage`) stores skills as
//! JSON files in a configurable directory.
//!
//! # Architecture
//!
//! - `SkillStorage` trait defines the storage interface
//! - `FileSkillStorage` implements file-based JSON storage
//! - `SkillIndex` maintains an in-memory index for fast lookups
//!
//! # Example
//!
//! ```
//! use nebula_marketplace::persistence::{FileSkillStorage, SkillStorage, SkillIndex};
//! use nebula_marketplace::types::{Skill, SkillMetadata, SkillManifest, SkillVersion};
//! use std::path::PathBuf;
//!
//! // Create a file-based storage with a temporary directory
//! let storage = FileSkillStorage::new(PathBuf::from("/tmp/skills"));
//!
//! // Create a skill
//! let metadata = SkillMetadata::new(
//!     "test-skill".to_string(),
//!     "A test skill".to_string(),
//!     "author".to_string(),
//! )
//! .with_version(SkillVersion::new(1, 0, 0));
//! let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
//! let skill = Skill::new("test-1".to_string(), metadata, manifest);
//!
//! // Save the skill
//! storage.save_skill(&skill).unwrap();
//!
//! // Load the skill
//! let loaded = storage.load_skill("test-1").unwrap();
//! assert_eq!(loaded.id, "test-1");
//!
//! // List all skills
//! let skills = storage.list_skills().unwrap();
//! assert_eq!(skills.len(), 1);
//!
//! // Update the skill
//! let mut updated_skill = loaded.clone();
//! updated_skill.metadata.version = SkillVersion::new(1, 1, 0);
//! storage.update_skill(&updated_skill).unwrap();
//!
//! // Delete the skill
//! storage.delete_skill("test-1").unwrap();
//! ```

use crate::types::Skill;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors specific to persistence operations.
#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Storage directory not set")]
    DirectoryNotSet,

    #[error("Storage directory does not exist: {0}")]
    DirectoryNotExist(PathBuf),

    #[error("Failed to create storage directory: {0}")]
    DirectoryCreateError(std::io::Error),

    #[error("Skill file is corrupted: {0}")]
    CorruptedFile(String),

    #[error("Index is out of sync with storage")]
    IndexOutOfSync,

    #[error("Concurrent modification detected")]
    ConcurrentModification,

    #[error("Storage backend error: {0}")]
    BackendError(String),
}

/// Result type for persistence operations.
pub type PersistenceResult<T> = std::result::Result<T, PersistenceError>;

/// Trait defining the storage interface for skills.
///
/// This trait abstracts the storage mechanism, allowing different backends
/// (file-based, database, in-memory, etc.) to be used interchangeably.
pub trait SkillStorage: Send + Sync {
    /// Save a skill to storage.
    ///
    /// If the skill already exists, it will be overwritten.
    fn save_skill(&self, skill: &Skill) -> PersistenceResult<()>;

    /// Load a skill from storage by its ID.
    fn load_skill(&self, id: &str) -> PersistenceResult<Skill>;

    /// Delete a skill from storage by its ID.
    fn delete_skill(&self, id: &str) -> PersistenceResult<()>;

    /// List all skills in storage.
    fn list_skills(&self) -> PersistenceResult<Vec<Skill>>;

    /// Update an existing skill.
    ///
    /// This is semantically similar to `save_skill` but may perform additional
    /// validation or version checking in some implementations.
    fn update_skill(&self, skill: &Skill) -> PersistenceResult<()>;

    /// Check if a skill exists in storage.
    fn exists(&self, id: &str) -> PersistenceResult<bool>;

    /// Get the count of skills in storage.
    fn count(&self) -> PersistenceResult<usize>;

    /// Clear all skills from storage.
    fn clear(&self) -> PersistenceResult<()>;

    /// Get the storage configuration.
    fn config(&self) -> StorageConfig;
}

/// Configuration for storage backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// The storage directory path (for file-based storage).
    pub directory: Option<PathBuf>,

    /// Whether to create the directory if it doesn't exist.
    pub create_if_missing: bool,

    /// Whether to sync to disk immediately after writes.
    pub sync_on_write: bool,

    /// Maximum number of skills to cache in memory.
    pub cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            directory: None,
            create_if_missing: true,
            sync_on_write: true,
            cache_size: 1000,
        }
    }
}

impl StorageConfig {
    /// Create a new storage config with the given directory.
    pub fn with_directory(mut self, directory: PathBuf) -> Self {
        self.directory = Some(directory);
        self
    }

    /// Set whether to create the directory if it doesn't exist.
    pub fn with_create_if_missing(mut self, create: bool) -> Self {
        self.create_if_missing = create;
        self
    }

    /// Set whether to sync to disk immediately after writes.
    pub fn with_sync_on_write(mut self, sync: bool) -> Self {
        self.sync_on_write = sync;
        self
    }

    /// Set the cache size.
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }
}

/// File-based skill storage implementation.
///
/// Stores each skill as a separate JSON file in a configurable directory.
/// The file name is derived from the skill ID with a `.json` extension.
#[derive(Debug)]
pub struct FileSkillStorage {
    config: StorageConfig,
    index: Arc<RwLock<SkillIndex>>,
}

impl FileSkillStorage {
    /// Create a new file-based storage with the given directory.
    pub fn new(directory: PathBuf) -> Self {
        Self::with_config(
            StorageConfig::default().with_directory(directory),
        )
    }

    /// Create a new file-based storage with custom configuration.
    pub fn with_config(config: StorageConfig) -> Self {
        let storage = Self {
            config,
            index: Arc::new(RwLock::new(SkillIndex::new())),
        };

        // Ensure directory exists if configured to do so
        if let Some(dir) = storage.config.directory.as_ref() {
            if storage.config.create_if_missing && !dir.exists() {
                if let Err(e) = fs::create_dir_all(dir) {
                    eprintln!("Warning: Failed to create storage directory: {}", e);
                }
            }
        }

        // Sync index from disk
        if let Err(e) = storage.sync_from_disk() {
            eprintln!("Warning: Failed to sync index from disk: {}", e);
        }

        storage
    }

    /// Get the path to the skill file for a given skill ID.
    fn skill_file_path(&self, id: &str) -> PersistenceResult<PathBuf> {
        let dir = self
            .config
            .directory
            .as_ref()
            .ok_or(PersistenceError::DirectoryNotSet)?;

        if !dir.exists() {
            return Err(PersistenceError::DirectoryNotExist(dir.clone()));
        }

        // Sanitize the skill ID to prevent path traversal
        let sanitized_id = id.replace(['/', '\\', '.'], "_").replace("..", "_");
        Ok(dir.join(format!("{}.json", sanitized_id)))
    }

    /// Sync the in-memory index from disk.
    pub fn sync_from_disk(&self) -> PersistenceResult<()> {
        let dir = self
            .config
            .directory
            .as_ref()
            .ok_or(PersistenceError::DirectoryNotSet)?;

        if !dir.exists() {
            return Ok(());
        }

        let mut index = self
            .index
            .write()
            .map_err(|_| PersistenceError::ConcurrentModification)?;

        index.clear();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.read_skill_file(&path) {
                    Ok(skill) => {
                        index.insert(skill.id.clone(), skill);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read skill file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Sync the in-memory index to disk.
    pub fn sync_to_disk(&self) -> PersistenceResult<()> {
        let index = self
            .index
            .read()
            .map_err(|_| PersistenceError::ConcurrentModification)?;

        for skill in index.values() {
            self.write_skill_file(skill)?;
        }

        Ok(())
    }

    /// Read a skill from a file.
    fn read_skill_file(&self, path: &Path) -> PersistenceResult<Skill> {
        let content = fs::read_to_string(path)?;
        let skill: Skill = serde_json::from_str(&content)?;
        Ok(skill)
    }

    /// Write a skill to a file.
    fn write_skill_file(&self, skill: &Skill) -> PersistenceResult<()> {
        let path = self.skill_file_path(&skill.id)?;
        let content = serde_json::to_string_pretty(skill)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get a reference to the index.
    pub fn index(&self) -> &Arc<RwLock<SkillIndex>> {
        &self.index
    }

    /// Rebuild the index from disk.
    pub fn rebuild_index(&self) -> PersistenceResult<()> {
        self.sync_from_disk()
    }
}

impl SkillStorage for FileSkillStorage {
    fn save_skill(&self, skill: &Skill) -> PersistenceResult<()> {
        // Write to disk
        self.write_skill_file(skill)?;

        // Update index
        let mut index = self
            .index
            .write()
            .map_err(|_| PersistenceError::ConcurrentModification)?;
        index.insert(skill.id.clone(), skill.clone());

        Ok(())
    }

    fn load_skill(&self, id: &str) -> PersistenceResult<Skill> {
        // Try index first
        {
            let index = self
                .index
                .read()
                .map_err(|_| PersistenceError::ConcurrentModification)?;
            if let Some(skill) = index.get(id) {
                return Ok(skill.clone());
            }
        }

        // Fall back to disk
        let path = self.skill_file_path(id)?;
        if !path.exists() {
            return Err(PersistenceError::SkillNotFound(id.to_string()));
        }

        let skill = self.read_skill_file(&path)?;

        // Update index
        let mut index = self
            .index
            .write()
            .map_err(|_| PersistenceError::ConcurrentModification)?;
        index.insert(id.to_string(), skill.clone());

        Ok(skill)
    }

    fn delete_skill(&self, id: &str) -> PersistenceResult<()> {
        // Check if exists
        if !self.exists(id)? {
            return Err(PersistenceError::SkillNotFound(id.to_string()));
        }

        // Delete file
        let path = self.skill_file_path(id)?;
        if path.exists() {
            fs::remove_file(&path)?;
        }

        // Remove from index
        let mut index = self
            .index
            .write()
            .map_err(|_| PersistenceError::ConcurrentModification)?;
        index.remove(id);

        Ok(())
    }

    fn list_skills(&self) -> PersistenceResult<Vec<Skill>> {
        let index = self
            .index
            .read()
            .map_err(|_| PersistenceError::ConcurrentModification)?;
        Ok(index.values().cloned().collect())
    }

    fn update_skill(&self, skill: &Skill) -> PersistenceResult<()> {
        // Check if exists
        if !self.exists(&skill.id)? {
            return Err(PersistenceError::SkillNotFound(skill.id.clone()));
        }

        // Save (overwrite)
        self.save_skill(skill)
    }

    fn exists(&self, id: &str) -> PersistenceResult<bool> {
        // Check index first
        {
            let index = self
                .index
                .read()
                .map_err(|_| PersistenceError::ConcurrentModification)?;
            if index.contains_key(id) {
                return Ok(true);
            }
        }

        // Check disk
        let path = self.skill_file_path(id)?;
        Ok(path.exists())
    }

    fn count(&self) -> PersistenceResult<usize> {
        let index = self
            .index
            .read()
            .map_err(|_| PersistenceError::ConcurrentModification)?;
        Ok(index.len())
    }

    fn clear(&self) -> PersistenceResult<()> {
        let dir = self
            .config
            .directory
            .as_ref()
            .ok_or(PersistenceError::DirectoryNotSet)?;

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    fs::remove_file(&path)?;
                }
            }
        }

        let mut index = self
            .index
            .write()
            .map_err(|_| PersistenceError::ConcurrentModification)?;
        index.clear();

        Ok(())
    }

    fn config(&self) -> StorageConfig {
        self.config.clone()
    }
}

/// In-memory index for fast skill lookups.
///
/// Maintains multiple indexes for different lookup patterns:
/// - By ID (primary)
/// - By name
/// - By category
/// - By author
#[derive(Debug, Clone, Default)]
pub struct SkillIndex {
    /// Primary index by skill ID.
    by_id: HashMap<String, Skill>,

    /// Index by skill name.
    by_name: HashMap<String, Vec<String>>,

    /// Index by category.
    by_category: HashMap<String, Vec<String>>,

    /// Index by author.
    by_author: HashMap<String, Vec<String>>,

    /// Index by tags.
    by_tag: HashMap<String, Vec<String>>,
}

impl SkillIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a skill into the index.
    pub fn insert(&mut self, id: String, skill: Skill) {
        // Remove old entries if updating
        if let Some(existing) = self.by_id.get(&id).cloned() {
            self.remove_by_skill(&existing);
        }

        // Index by ID
        self.by_id.insert(id.clone(), skill.clone());

        // Index by name
        self.by_name
            .entry(skill.metadata.name.clone())
            .or_default()
            .push(id.clone());

        // Index by category
        for category in &skill.metadata.categories {
            let cat_str = format!("{:?}", category);
            self.by_category
                .entry(cat_str)
                .or_default()
                .push(id.clone());
        }

        // Index by author
        self.by_author
            .entry(skill.metadata.author.clone())
            .or_default()
            .push(id.clone());

        // Index by tags
        for tag in &skill.metadata.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .push(id.clone());
        }
    }

    /// Remove a skill from the index by its skill data.
    fn remove_by_skill(&mut self, skill: &Skill) {
        let id = &skill.id;

        // Remove from by_id
        self.by_id.remove(id);

        // Remove from by_name
        if let Some(ids) = self.by_name.get_mut(&skill.metadata.name) {
            ids.retain(|i| i != id);
            if ids.is_empty() {
                self.by_name.remove(&skill.metadata.name);
            }
        }

        // Remove from by_category
        for category in &skill.metadata.categories {
            let cat_str = format!("{:?}", category);
            if let Some(ids) = self.by_category.get_mut(&cat_str) {
                ids.retain(|i| i != id);
                if ids.is_empty() {
                    self.by_category.remove(&cat_str);
                }
            }
        }

        // Remove from by_author
        if let Some(ids) = self.by_author.get_mut(&skill.metadata.author) {
            ids.retain(|i| i != id);
            if ids.is_empty() {
                self.by_author.remove(&skill.metadata.author);
            }
        }

        // Remove from by_tag
        for tag in &skill.metadata.tags {
            if let Some(ids) = self.by_tag.get_mut(tag) {
                ids.retain(|i| i != id);
                if ids.is_empty() {
                    self.by_tag.remove(tag);
                }
            }
        }
    }

    /// Remove a skill from the index by its ID.
    pub fn remove(&mut self, id: &str) -> Option<Skill> {
        if let Some(skill) = self.by_id.remove(id) {
            self.remove_by_skill(&skill);
            return Some(skill);
        }
        None
    }

    /// Get a skill by ID.
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.by_id.get(id)
    }

    /// Get skills by name.
    pub fn get_by_name(&self, name: &str) -> Vec<&Skill> {
        self.by_name
            .get(name)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get skills by category.
    pub fn get_by_category(&self, category: &str) -> Vec<&Skill> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get skills by author.
    pub fn get_by_author(&self, author: &str) -> Vec<&Skill> {
        self.by_author
            .get(author)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get skills by tag.
    pub fn get_by_tag(&self, tag: &str) -> Vec<&Skill> {
        self.by_tag
            .get(tag)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all skills.
    pub fn values(&self) -> impl Iterator<Item = &Skill> {
        self.by_id.values()
    }

    /// Get all skill IDs.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.by_id.keys()
    }

    /// Get the number of skills in the index.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Check if the index contains a skill by ID.
    pub fn contains_key(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.by_id.clear();
        self.by_name.clear();
        self.by_category.clear();
        self.by_author.clear();
        self.by_tag.clear();
    }

    /// Search skills by multiple criteria.
    pub fn search(
        &self,
        name: Option<&str>,
        category: Option<&str>,
        author: Option<&str>,
        tags: &[&str],
    ) -> Vec<&Skill> {
        let mut results: Vec<&Skill> = self.by_id.values().collect();

        if let Some(name) = name {
            results.retain(|s| s.metadata.name.contains(name));
        }

        if let Some(category) = category {
            results.retain(|s| {
                s.metadata
                    .categories
                    .iter()
                    .any(|c| format!("{:?}", c).contains(category))
            });
        }

        if let Some(author) = author {
            results.retain(|s| s.metadata.author.contains(author));
        }

        for tag in tags {
            results.retain(|s| s.metadata.tags.iter().any(|t| t.contains(tag)));
        }

        results
    }
}

/// A generic wrapper that provides a high-level API for skill persistence.
///
/// This struct combines storage and indexing for convenient access.
#[derive(Debug)]
pub struct SkillPersistence<S: SkillStorage> {
    storage: S,
}

impl<S: SkillStorage> SkillPersistence<S> {
    /// Create a new persistence layer with the given storage backend.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Get a reference to the underlying storage.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Save a skill.
    pub fn save(&self, skill: &Skill) -> PersistenceResult<()> {
        self.storage.save_skill(skill)
    }

    /// Load a skill.
    pub fn load(&self, id: &str) -> PersistenceResult<Skill> {
        self.storage.load_skill(id)
    }

    /// Delete a skill.
    pub fn delete(&self, id: &str) -> PersistenceResult<()> {
        self.storage.delete_skill(id)
    }

    /// List all skills.
    pub fn list(&self) -> PersistenceResult<Vec<Skill>> {
        self.storage.list_skills()
    }

    /// Update a skill.
    pub fn update(&self, skill: &Skill) -> PersistenceResult<()> {
        self.storage.update_skill(skill)
    }

    /// Check if a skill exists.
    pub fn exists(&self, id: &str) -> PersistenceResult<bool> {
        self.storage.exists(id)
    }

    /// Get the count of skills.
    pub fn count(&self) -> PersistenceResult<usize> {
        self.storage.count()
    }

    /// Clear all skills.
    pub fn clear(&self) -> PersistenceResult<()> {
        self.storage.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SkillManifest, SkillMetadata, SkillVersion};
    use tempfile::tempdir;

    fn create_test_skill(id: &str, name: &str, version: SkillVersion) -> Skill {
        let metadata = SkillMetadata::new(
            name.to_string(),
            format!("Description for {}", name),
            "test-author".to_string(),
        )
        .with_version(version)
        .with_categories(vec![crate::types::SkillCategory::CodeGeneration])
        .with_tags(vec!["test".to_string(), "rust".to_string()]);

        let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
        Skill::new(id.to_string(), metadata, manifest)
    }

    #[test]
    fn test_storage_config_builder() {
        let config = StorageConfig::default()
            .with_directory(PathBuf::from("/tmp/test"))
            .with_create_if_missing(false)
            .with_sync_on_write(false)
            .with_cache_size(500);

        assert_eq!(config.directory, Some(PathBuf::from("/tmp/test")));
        assert!(!config.create_if_missing);
        assert!(!config.sync_on_write);
        assert_eq!(config.cache_size, 500);
    }

    #[test]
    fn test_file_storage_save_and_load() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        // Save
        storage.save_skill(&skill).unwrap();

        // Load
        let loaded = storage.load_skill("test-1").unwrap();
        assert_eq!(loaded.id, "test-1");
        assert_eq!(loaded.metadata.name, "Test Skill");
        assert_eq!(loaded.metadata.version.to_string(), "1.0.0");
    }

    #[test]
    fn test_file_storage_update() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        storage.save_skill(&skill).unwrap();

        let mut updated = skill.clone();
        updated.metadata.version = SkillVersion::new(2, 0, 0);
        storage.update_skill(&updated).unwrap();

        let loaded = storage.load_skill("test-1").unwrap();
        assert_eq!(loaded.metadata.version.to_string(), "2.0.0");
    }

    #[test]
    fn test_file_storage_delete() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        storage.save_skill(&skill).unwrap();

        assert!(storage.exists("test-1").unwrap());

        storage.delete_skill("test-1").unwrap();

        assert!(!storage.exists("test-1").unwrap());
        assert!(storage.load_skill("test-1").is_err());
    }

    #[test]
    fn test_file_storage_list() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill1 = create_test_skill("test-1", "Skill One", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-2", "Skill Two", SkillVersion::new(1, 0, 0));

        storage.save_skill(&skill1).unwrap();
        storage.save_skill(&skill2).unwrap();

        let skills = storage.list_skills().unwrap();
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn test_file_storage_count() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        assert_eq!(storage.count().unwrap(), 0);

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        storage.save_skill(&skill).unwrap();

        assert_eq!(storage.count().unwrap(), 1);
    }

    #[test]
    fn test_file_storage_clear() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill1 = create_test_skill("test-1", "Skill One", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-2", "Skill Two", SkillVersion::new(1, 0, 0));

        storage.save_skill(&skill1).unwrap();
        storage.save_skill(&skill2).unwrap();

        assert_eq!(storage.count().unwrap(), 2);

        storage.clear().unwrap();

        assert_eq!(storage.count().unwrap(), 0);
    }

    #[test]
    fn test_file_storage_exists() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        assert!(!storage.exists("test-1").unwrap());

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        storage.save_skill(&skill).unwrap();

        assert!(storage.exists("test-1").unwrap());
    }

    #[test]
    fn test_file_storage_load_not_found() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let result = storage.load_skill("nonexistent");
        assert!(matches!(result, Err(PersistenceError::SkillNotFound(_))));
    }

    #[test]
    fn test_file_storage_update_not_found() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        let result = storage.update_skill(&skill);
        assert!(matches!(result, Err(PersistenceError::SkillNotFound(_))));
    }

    #[test]
    fn test_file_storage_delete_not_found() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let result = storage.delete_skill("nonexistent");
        assert!(matches!(result, Err(PersistenceError::SkillNotFound(_))));
    }

    #[test]
    fn test_file_storage_sync_from_disk() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        storage.save_skill(&skill).unwrap();

        // Create a new storage instance pointing to the same directory
        let storage2 = FileSkillStorage::new(dir.path().to_path_buf());

        // Should have synced from disk
        assert_eq!(storage2.count().unwrap(), 1);
        let loaded = storage2.load_skill("test-1").unwrap();
        assert_eq!(loaded.id, "test-1");
    }

    #[test]
    fn test_skill_index_insert_and_get() {
        let mut index = SkillIndex::new();
        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill.clone());

        assert_eq!(index.len(), 1);
        assert!(index.contains_key("test-1"));
        assert_eq!(index.get("test-1").unwrap().id, "test-1");
    }

    #[test]
    fn test_skill_index_get_by_name() {
        let mut index = SkillIndex::new();
        let skill1 = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-2", "Test Skill", SkillVersion::new(2, 0, 0));

        index.insert("test-1".to_string(), skill1);
        index.insert("test-2".to_string(), skill2);

        let skills = index.get_by_name("Test Skill");
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn test_skill_index_get_by_category() {
        let mut index = SkillIndex::new();
        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill);

        let skills = index.get_by_category("CodeGeneration");
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn test_skill_index_get_by_author() {
        let mut index = SkillIndex::new();
        let skill1 = create_test_skill("test-1", "Skill One", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-2", "Skill Two", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill1);
        index.insert("test-2".to_string(), skill2);

        let skills = index.get_by_author("test-author");
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn test_skill_index_get_by_tag() {
        let mut index = SkillIndex::new();
        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill);

        let skills = index.get_by_tag("test");
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn test_skill_index_remove() {
        let mut index = SkillIndex::new();
        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill);
        assert_eq!(index.len(), 1);

        index.remove("test-1");
        assert_eq!(index.len(), 0);
        assert!(!index.contains_key("test-1"));
    }

    #[test]
    fn test_skill_index_search() {
        let mut index = SkillIndex::new();
        let skill1 = create_test_skill("test-1", "Rust Skill", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-2", "Python Skill", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill1);
        index.insert("test-2".to_string(), skill2);

        let results = index.search(Some("Rust"), None, None, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Rust Skill");

        let results = index.search(None, None, None, &["test"]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_skill_index_clear() {
        let mut index = SkillIndex::new();
        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        index.insert("test-1".to_string(), skill);
        assert_eq!(index.len(), 1);

        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_skill_persistence_wrapper() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());
        let persistence = SkillPersistence::new(storage);

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));

        persistence.save(&skill).unwrap();
        assert!(persistence.exists("test-1").unwrap());

        let loaded = persistence.load("test-1").unwrap();
        assert_eq!(loaded.id, "test-1");

        assert_eq!(persistence.count().unwrap(), 1);

        let skills = persistence.list().unwrap();
        assert_eq!(skills.len(), 1);

        persistence.delete("test-1").unwrap();
        assert!(!persistence.exists("test-1").unwrap());
    }

    #[test]
    fn test_file_storage_with_custom_config() {
        let dir = tempdir().unwrap();
        let config = StorageConfig::default()
            .with_directory(dir.path().to_path_buf())
            .with_create_if_missing(true)
            .with_sync_on_write(true)
            .with_cache_size(100);

        let storage = FileSkillStorage::with_config(config);

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        storage.save_skill(&skill).unwrap();

        let loaded = storage.load_skill("test-1").unwrap();
        assert_eq!(loaded.id, "test-1");
    }

    #[test]
    fn test_storage_without_directory() {
        let config = StorageConfig::default();
        let storage = FileSkillStorage::with_config(config);

        let skill = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        let result = storage.save_skill(&skill);
        assert!(matches!(result, Err(PersistenceError::DirectoryNotSet)));
    }

    #[test]
    fn test_skill_index_update_existing() {
        let mut index = SkillIndex::new();
        let skill1 = create_test_skill("test-1", "Test Skill", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-1", "Test Skill Updated", SkillVersion::new(2, 0, 0));

        index.insert("test-1".to_string(), skill1);
        assert_eq!(index.len(), 1);

        index.insert("test-1".to_string(), skill2);
        assert_eq!(index.len(), 1);
        assert_eq!(index.get("test-1").unwrap().metadata.version.to_string(), "2.0.0");
    }

    #[test]
    fn test_multiple_skills_same_name() {
        let dir = tempdir().unwrap();
        let storage = FileSkillStorage::new(dir.path().to_path_buf());

        let skill1 = create_test_skill("test-1", "Same Name", SkillVersion::new(1, 0, 0));
        let skill2 = create_test_skill("test-2", "Same Name", SkillVersion::new(2, 0, 0));

        storage.save_skill(&skill1).unwrap();
        storage.save_skill(&skill2).unwrap();

        assert_eq!(storage.count().unwrap(), 2);

        let by_name = storage.list_skills().unwrap();
        let same_name_count = by_name.iter().filter(|s| s.metadata.name == "Same Name").count();
        assert_eq!(same_name_count, 2);
    }

    #[test]
    fn test_persistence_error_display() {
        let err = PersistenceError::SkillNotFound("test".to_string());
        assert_eq!(err.to_string(), "Skill not found: test");

        let err = PersistenceError::DirectoryNotSet;
        assert_eq!(err.to_string(), "Storage directory not set");
    }
}