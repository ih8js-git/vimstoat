use std::{
    any::type_name,
    fmt::{self, Debug},
    marker::PhantomData,
    path::PathBuf,
};

use pickledb::PickleDb;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    Result,
    error::{CacheError, IdError},
};

pub const DB_FILE: &str = "cache.db";

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Id<T> {
    inner: [u8; 26],
    _marker: PhantomData<T>,
}

#[allow(unused)]
impl<T> Id<T> {
    pub fn new(id: &str) -> Result<Self> {
        let inner: [u8; 26] = id
            .as_bytes()
            .try_into()
            .map_err(|_| IdError::InvalidSize(id.len()))?;
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(self.bytes()).expect("Failed to convert Id bytes to str!")
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Id<{}>({})",
            type_name::<T>().split("::").last().unwrap_or(""),
            self.as_str()
        )
    }
}

#[allow(unused)]
pub struct CacheStore {
    db: PickleDb,
    path: PathBuf,
}

#[allow(unused)]
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
                &path,
                pickledb::PickleDbDumpPolicy::AutoDump,
                pickledb::SerializationMethod::Bin,
            )
            .map_err(CacheError::DbError)?
        } else {
            PickleDb::new(
                &path,
                pickledb::PickleDbDumpPolicy::AutoDump,
                pickledb::SerializationMethod::Bin,
            )
        };

        Ok(Self { db, path })
    }

    pub fn set<V: Serialize + Debug>(&mut self, id: Id<V>, value: &V) -> Result<()> {
        let key = Self::build_key::<V>(id)?;

        log::info!("Setting key-value to cache: k: {:?}, v: {:?}", key, value);
        self.db.set(key.as_ref(), value)?;
        Ok(())
    }

    pub fn get<V: DeserializeOwned>(&self, id: Id<V>) -> Option<V> {
        let key = Self::build_key::<V>(id).ok()?;

        log::info!("Getting value from key in cache: k: {:?}", key);
        self.db.get::<V>(key.as_ref())
    }

    pub fn exists<V>(&self, id: Id<V>) -> Result<bool> {
        let key = Self::build_key::<V>(id)?;

        Ok(self.db.exists(key.as_ref()))
    }

    pub fn remove<V>(&mut self, id: Id<V>) -> Result<bool> {
        let key = Self::build_key::<V>(id)?;

        log::info!("Deleting value from key in cache: k: {:?}", key);
        let key_str = key.as_ref();

        if self.db.exists(key_str) {
            self.db.rem(key_str)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn build_key<V>(id: Id<V>) -> Result<String> {
        let full_name = type_name::<V>();
        let short_name = full_name.split("::").last().unwrap_or(full_name);
        Ok(format!("{}:{}", short_name.to_lowercase(), id.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct MockUser {
        name: String,
        age: u8,
    }

    fn setup_temporary_cache() -> (CacheStore, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create a temporary directory");
        let path = temp_dir.path().join("test_cache.db");

        let db = PickleDb::new(
            &path,
            pickledb::PickleDbDumpPolicy::AutoDump,
            pickledb::SerializationMethod::Bin,
        );

        (CacheStore { db, path }, temp_dir)
    }

    #[test]
    fn test_id_new_valid() {
        let valid_str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let id_res = Id::<MockUser>::new(valid_str);

        assert!(id_res.is_ok());
        let id = id_res.unwrap();
        assert_eq!(id.as_str(), valid_str);
        assert_eq!(id.bytes(), valid_str.as_bytes());
    }

    #[test]
    fn test_id_new_invalid_size() {
        let short_str = "01ARZ3NDEKTSV4RRFFQ69G5FA";
        let id_err = Id::<MockUser>::new(short_str);
        assert!(id_err.is_err());

        let long_str = "01ARZ3NDEKTSV4RRFFQ69G5FAVV";
        let id_err = Id::<MockUser>::new(long_str);
        assert!(id_err.is_err());
    }

    #[test]
    fn test_id_debug_formatting() {
        let valid_str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let id = Id::<MockUser>::new(valid_str).unwrap();

        let debug_format = format!("{:?}", id);
        assert_eq!(debug_format, "Id<MockUser>(01ARZ3NDEKTSV4RRFFQ69G5FAV)");
    }

    #[test]
    fn test_cache_store_lifecycle() {
        let (mut store, _temp_dir) = setup_temporary_cache();

        let raw_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let id = Id::<MockUser>::new(raw_id).unwrap();
        let user = MockUser {
            name: "Alice".to_string(),
            age: 30,
        };

        assert!(!store.exists(id.clone()).unwrap(), "Key should not exist");
        assert!(store.get(id.clone()).is_none(), "Get should return None");

        let set_res = store.set(id.clone(), &user);
        assert!(set_res.is_ok(), "Insertion failed: {:?}", set_res);

        assert!(
            store.exists(id.clone()).unwrap(),
            "Key should exist after set"
        );

        let cached_user = store.get(id.clone());
        assert!(cached_user.is_some(), "Value should have been retrieved");
        assert_eq!(
            cached_user.unwrap(),
            user,
            "Retrieved value does not match the inserted one"
        );

        let remove_res = store.remove(id.clone());
        assert!(remove_res.is_ok());
        assert!(remove_res.unwrap(), "Remove should have returned true");

        assert!(
            !store.exists(id.clone()).unwrap(),
            "Key should no longer exist after removal"
        );

        let remove_again = store.remove(id.clone()).unwrap();
        assert!(
            !remove_again,
            "Remove should have returned false for a non-existent key"
        );
    }
}
