// This will do simple Thread safe, concurrent CRUD operations on an in-memory HashMap, with BufReader and BufWriter for reading and writing data.
// This is not going to be whole ass Storage engine, just simple Buffer Reader and Writer, I swear 🙃, Please don't get too involved (Maybe later use Btree)
// Lets begin...

//! # Storage Engine
//!
//! A lightweight, thread-safe storage engine with JSON persistence.
//!
//! ## Features
//! - **Thread-safe**: Uses Arc<RwLock<T>> for concurrent access
//! - **Fast reads**: Uses memmap2 for memory-mapped file access
//! - **Efficient writes**: Uses BufWriter for buffered writing
//! - **Generic**: Works with any types that implement Serialize + Deserialize
//! - **Persistent**: Automatically saves to JSON files

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Thread-safe DataStore with in-memory HashMap and persistent JSON storage
/// Uses Arc<RwLock<T>> for concurrent access and memmap2 for fast reads
#[derive(Clone)]
pub struct DataStore<K, V>
where
    K: Eq + Hash + Clone + Serialize + for<'de> Deserialize<'de>,
    V: Clone + Serialize + for<'de> Deserialize<'de>,
{
    /// In-memory storage with thread-safety
    data: Arc<RwLock<HashMap<K, V>>>,
    /// File path for persistence
    path: PathBuf,
}

impl<K, V> DataStore<K, V>
where
    K: Eq + Hash + Clone + Serialize + for<'de> Deserialize<'de>,
    V: Clone + Serialize + for<'de> Deserialize<'de>,
{
    /// Create a new DataStore with the given file path
    pub fn new(path: PathBuf) -> Result<Self> {
        let data = Arc::new(RwLock::new(HashMap::new()));
        let store = DataStore { data, path };

        // Load existing data if file exists
        if store.path.exists() {
            store.load_from_disk()?;
        }

        Ok(store)
    }

    /// Insert or update a key-value pair
    pub fn insert(&self, key: K, value: V) -> Result<Option<V>> {
        let mut data = self
            .data
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        let old_value = data.insert(key, value);
        drop(data); // Release lock before disk I/O

        // Persist to disk
        self.save_to_disk()?;

        Ok(old_value)
    }

    /// Get a value by key
    pub fn get(&self, key: &K) -> Result<Option<V>> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.get(key).cloned())
    }

    /// Delete a key-value pair
    pub fn delete(&self, key: &K) -> Result<Option<V>> {
        let mut data = self
            .data
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        let removed = data.remove(key);
        drop(data); // Release lock before disk I/O

        if removed.is_some() {
            self.save_to_disk()?;
        }

        Ok(removed)
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: &K) -> Result<bool> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.contains_key(key))
    }

    /// Get all keys
    pub fn keys(&self) -> Result<Vec<K>> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.keys().cloned().collect())
    }

    /// Get all values
    pub fn values(&self) -> Result<Vec<V>> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.values().cloned().collect())
    }

    /// Get all key-value pairs
    pub fn entries(&self) -> Result<Vec<(K, V)>> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    /// Get the number of entries
    pub fn len(&self) -> Result<usize> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.len())
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> Result<bool> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.is_empty())
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        data.clear();
        drop(data);

        self.save_to_disk()?;

        Ok(())
    }

    /// Save data to disk using BufWriter for efficient writing (Explicitly)
    pub fn save_to_disk(&self) -> Result<()> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create parent directory")?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .context("Failed to open file for writing")?;

        let mut writer = BufWriter::new(file);

        serde_json::to_writer_pretty(&mut writer, &*data)
            .context("Failed to serialize data to JSON")?;

        writer.flush().context("Failed to flush writer")?;

        Ok(())
    }

    /// Load data from disk using memmap2 for fast reading (Explicitly)
    pub fn load_from_disk(&self) -> Result<()> {
        let file = File::open(&self.path).context("Failed to open file for reading")?;

        // Use memmap2 for fast memory-mapped file access
        let mmap = unsafe { memmap2::Mmap::map(&file).context("Failed to create memory map")? };

        // Deserialize from the memory-mapped data
        let loaded_data: HashMap<K, V> =
            serde_json::from_slice(&mmap).context("Failed to deserialize JSON data")?;

        let mut data = self
            .data
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        *data = loaded_data;

        Ok(())
    }

    /// Reload data from disk (useful for synchronization)
    pub fn reload(&self) -> Result<()> {
        if self.path.exists() {
            self.load_from_disk()
        } else {
            Ok(())
        }
    }

    /// Get a snapshot of all data (useful for batch operations)
    pub fn snapshot(&self) -> Result<HashMap<K, V>> {
        let data = self
            .data
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(data.clone())
    }

    /// Batch insert multiple key-value pairs
    pub fn batch_insert(&self, entries: Vec<(K, V)>) -> Result<()> {
        let mut data = self
            .data
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        for (key, value) in entries {
            data.insert(key, value);
        }

        drop(data);

        self.save_to_disk()?;

        Ok(())
    }
}

#[test]
fn test_basic_operations() -> Result<()> {
    use std::env;
    let temp_path = env::temp_dir().join("test_store_basic.json");

    let _ = std::fs::remove_file(&temp_path);

    let store: DataStore<String, String> = DataStore::new(temp_path.clone())?;

    store.insert("key1".to_string(), "value1".to_string())?;
    store.insert("key2".to_string(), "value2".to_string())?;

    assert_eq!(store.get(&"key1".to_string())?, Some("value1".to_string()));
    assert_eq!(store.get(&"key2".to_string())?, Some("value2".to_string()));

    assert!(store.contains_key(&"key1".to_string())?);
    assert!(!store.contains_key(&"key3".to_string())?);

    assert_eq!(store.len()?, 2);

    let removed = store.delete(&"key1".to_string())?;
    assert_eq!(removed, Some("value1".to_string()));
    assert_eq!(store.len()?, 1);

    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}

#[test]
fn test_persistence() -> Result<()> {
    use std::env;
    let temp_path = env::temp_dir().join("test_store_persistence.json");

    let _ = std::fs::remove_file(&temp_path);

    {
        let store: DataStore<String, i32> = DataStore::new(temp_path.clone())?;
        store.insert("counter".to_string(), 42)?;
        store.insert("score".to_string(), 100)?;
    } // Drop store

    // Load from disk in a new instance
    {
        let store: DataStore<String, i32> = DataStore::new(temp_path.clone())?;
        assert_eq!(store.get(&"counter".to_string())?, Some(42));
        assert_eq!(store.get(&"score".to_string())?, Some(100));
    }

    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}

#[test]
fn test_batch_operations() -> Result<()> {
    use std::env;
    let temp_path = env::temp_dir().join("test_store_batch.json");

    let _ = std::fs::remove_file(&temp_path);

    let store: DataStore<u32, String> = DataStore::new(temp_path.clone())?;

    // Batch insert
    let batch = vec![
        (1, "one".to_string()),
        (2, "two".to_string()),
        (3, "three".to_string()),
    ];
    store.batch_insert(batch)?;

    assert_eq!(store.len()?, 3);
    assert_eq!(store.get(&2)?, Some("two".to_string()));

    // Test snapshot
    let snapshot = store.snapshot()?;
    assert_eq!(snapshot.len(), 3);

    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}

#[test]
fn test_concurrent_access() -> Result<()> {
    use std::env;
    use std::sync::Arc;
    use std::thread;

    let temp_path = env::temp_dir().join("test_store_concurrent.json");

    let _ = std::fs::remove_file(&temp_path);

    let store: Arc<DataStore<u64, u64>> = Arc::new(DataStore::new(temp_path.clone())?);

    let mut handles = vec![];

    // Spawn multiple threads
    for i in 0..10 {
        let store_clone = Arc::clone(&store);
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let key = i * 10 + j;
                let _ = store_clone.insert(key, key * 2);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify
    assert_eq!(store.len()?, 100);

    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}
