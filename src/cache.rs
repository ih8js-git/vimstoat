use std::fmt::Debug;

use pickledb::PickleDb;
use serde::{Serialize, de::DeserializeOwned};

use crate::{Result, error::CacheError};

pub const DB_FILE: &str = "cache.db";

pub struct CacheStore {
    db: PickleDb,
}

impl CacheStore {
    pub fn new() -> Result<Self> {
        let path = if let Some(mut p) = dirs::cache_dir() {
            p.push(env!("CARGO_PKG_NAME"));
            p.push(DB_FILE);
            p
        } else {
            return Err(
                CacheError::DirNotFound("Couldn't find OS cache directory.".to_string()).into(),
            );
        };

        log::info!("Loading cache from file: {:?}", path.to_string_lossy());

        let db = if path.exists() {
            PickleDb::load(
                path,
                pickledb::PickleDbDumpPolicy::AutoDump,
                pickledb::SerializationMethod::Bin,
            )?
        } else {
            PickleDb::new(
                path,
                pickledb::PickleDbDumpPolicy::AutoDump,
                pickledb::SerializationMethod::Bin,
            )
        };

        Ok(Self { db })
    }

    pub fn set<K, V>(&mut self, key: K, value: &V) -> Result<()>
    where
        K: AsRef<str> + Debug,
        V: Serialize + Debug,
    {
        log::info!("Setting key-value to cache: k: {:?}, v: {:?}", key, value);
        self.db.set(key.as_ref(), value)?;
        Ok(())
    }

    pub fn get<K, V>(&self, key: K) -> Option<V>
    where
        K: AsRef<str> + Debug,
        V: DeserializeOwned,
    {
        log::info!("Getting value from key in cache: k: {:?}", key);
        self.db.get::<V>(key.as_ref())
    }

    pub fn exists<K: AsRef<str>>(&self, key: K) -> bool {
        self.db.exists(key.as_ref())
    }

    pub fn remove<K: AsRef<str> + Debug>(&mut self, key: K) -> Result<bool> {
        log::info!("Deleting value from key in cache: k: {:?}", key);
        let key_str = key.as_ref();
        if self.db.exists(key_str) {
            self.db.rem(key_str)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
