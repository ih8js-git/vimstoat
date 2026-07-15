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
