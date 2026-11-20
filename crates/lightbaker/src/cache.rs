//! Baked tiles and probe chunks kept by their hash (deps.rs): a tile whose hash is found is not
//! baked again. Where they are kept is a [`TileStore`]: files (native, zstd-compressed, [`FsStore`]) or
//! memory (any platform, [`MemoryStore`]); a missing or unreadable entry is a miss.

use std::collections::BTreeMap;
use std::sync::Mutex;

use anyhow::Result;
use bytemuck::Pod;

/// Where baked tiles (`kind` "tile") and probe chunks ("probes") are kept between bakes, by hash.
pub trait TileStore: Sync {
    fn get(&self, kind: &str, hash: &blake3::Hash) -> Option<Vec<u8>>;
    fn put(&self, kind: &str, hash: &blake3::Hash, bytes: &[u8]) -> Result<()>;
}

/// A bake's view of its store, if it has one.
pub struct Cache<'a> {
    store: Option<&'a dyn TileStore>,
}

impl<'a> Cache<'a> {
    pub fn new(store: Option<&'a dyn TileStore>) -> Result<Cache<'a>> {
        Ok(Cache { store })
    }

    pub fn get(&self, kind: &str, hash: &blake3::Hash) -> Option<Vec<u8>> {
        self.store?.get(kind, hash)
    }

    pub fn put(&self, kind: &str, hash: &blake3::Hash, bytes: &[u8]) -> Result<()> {
        match self.store {
            Some(s) => s.put(kind, hash, bytes),
            None => Ok(()),
        }
    }
}

/// Entries as files `{kind}-{hash}.bin` in a directory, zstd-compressed (upstream's cache format).
#[cfg(feature = "native")]
pub struct FsStore {
    dir: std::path::PathBuf,
}

#[cfg(feature = "native")]
impl FsStore {
    pub fn new(dir: &std::path::Path) -> Result<FsStore> {
        std::fs::create_dir_all(dir)?;
        Ok(FsStore { dir: dir.to_path_buf() })
    }
}

#[cfg(feature = "native")]
impl TileStore for FsStore {
    fn get(&self, kind: &str, hash: &blake3::Hash) -> Option<Vec<u8>> {
        let f = self.dir.join(format!("{kind}-{}.bin", hash.to_hex()));
        zstd::decode_all(std::fs::read(f).ok()?.as_slice()).ok()
    }

    fn put(&self, kind: &str, hash: &blake3::Hash, bytes: &[u8]) -> Result<()> {
        let f = self.dir.join(format!("{kind}-{}.bin", hash.to_hex()));
        let tmp = f.with_extension("tmp");
        std::fs::write(&tmp, zstd::encode_all(bytes, 3)?)?;
        std::fs::rename(tmp, f)?;
        Ok(())
    }
}

/// Entries in memory, uncompressed; lives as long as the store (the web keeps one per baker).
#[derive(Default)]
pub struct MemoryStore {
    entries: Mutex<BTreeMap<(String, [u8; 32]), Vec<u8>>>,
}

impl MemoryStore {
    pub fn new() -> MemoryStore {
        MemoryStore::default()
    }
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The bytes held, for memory budgets.
    pub fn bytes(&self) -> usize {
        self.entries.lock().unwrap().values().map(Vec::len).sum()
    }
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl TileStore for MemoryStore {
    fn get(&self, kind: &str, hash: &blake3::Hash) -> Option<Vec<u8>> {
        self.entries.lock().unwrap().get(&(kind.to_string(), *hash.as_bytes())).cloned()
    }

    fn put(&self, kind: &str, hash: &blake3::Hash, bytes: &[u8]) -> Result<()> {
        self.entries.lock().unwrap().insert((kind.to_string(), *hash.as_bytes()), bytes.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trip() {
        let s = MemoryStore::new();
        let (a, b) = (blake3::hash(b"a"), blake3::hash(b"b"));
        let cache = Cache::new(Some(&s)).unwrap();
        assert!(cache.get("tile", &a).is_none());
        cache.put("tile", &a, &[1, 2, 3]).unwrap();
        assert_eq!(cache.get("tile", &a), Some(vec![1, 2, 3]));
        assert!(cache.get("probes", &a).is_none(), "kinds are separate");
        assert!(cache.get("tile", &b).is_none());
        assert_eq!((s.len(), s.bytes()), (1, 3));
        assert!(Cache::new(None).unwrap().get("tile", &a).is_none());
    }
}
/// Arrays laid end to end, each preceded by its byte length.
#[derive(Default)]
pub struct Pack(pub Vec<u8>);

impl Pack {
    pub fn put<T: Pod>(mut self, v: &[T]) -> Self {
        let b: &[u8] = bytemuck::cast_slice(v);
        self.0.extend_from_slice(&(b.len() as u64).to_le_bytes());
        self.0.extend_from_slice(b);
        self
    }
}

pub struct Unpack<'a>(&'a [u8]);

impl<'a> Unpack<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Unpack(b)
    }
    pub fn take<T: Pod>(&mut self) -> Option<Vec<T>> {
        let n = u64::from_le_bytes(self.0.get(..8)?.try_into().ok()?) as usize;
        let b = self.0.get(8..8 + n)?;
        self.0 = &self.0[8 + n..];
        Some(bytemuck::pod_collect_to_vec(b))
    }
}
