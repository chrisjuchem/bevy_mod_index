use bevy::platform::collections::{
    hash_map::HashMap,
    hash_set::{HashSet, Iter},
};
use std::hash::Hash;

#[cfg(feature = "reflect")]
use bevy::reflect::Reflect;

/// Map where a key can have multiple values, but a value can only exist for one key at a time.
/// Re-inserting a value is a no-op if it already exists under the same key, otherwise the value is
/// removed from under its present key and added under the new key.
#[cfg_attr(feature = "reflect", derive(Reflect))]
pub struct UniqueMultiMap<K, V> {
    map: HashMap<K, HashSet<V>>,
    rev_map: HashMap<V, K>,
}

impl<K, V> Default for UniqueMultiMap<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            rev_map: Default::default(),
        }
    }
}

impl<K, V> UniqueMultiMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Hash + Eq + Clone,
{
    pub fn get(&self, k: &K) -> impl Iterator<Item = &V> {
        MultiMapValueIter {
            inner: self.map.get(k).map(|hashset| hashset.iter()),
        }
    }

    /// Returns value's old key
    // Todo: don't rely on clone
    pub fn insert(&mut self, new_k: &K, v: V) -> Option<K> {
        let maybe_old_k = self.rev_map.insert(v.clone(), new_k.clone());

        if let Some(old_k) = &maybe_old_k {
            // insert value into same key: no-op
            if old_k == new_k {
                return maybe_old_k;
            }

            // remove old value; its key must exist according to rev_map
            self.purge_from_forward(old_k, &v, "insert");
        }
        // insert new value
        self.map.get_mut_or_insert_default(new_k).insert(v);

        maybe_old_k
    }

    /// Returns value's old key
    pub fn remove(&mut self, v: &V) -> Option<K> {
        let maybe_old_k = self.rev_map.remove(v);

        if let Some(old_k) = &maybe_old_k {
            self.purge_from_forward(old_k, v, "remove");
        }

        maybe_old_k
    }

    // Removes v from k's set, removing the set completely if it would be empty
    // Panics if k is not in the forward map.
    fn purge_from_forward(&mut self, k: &K, v: &V, fn_name: &str) {
        let old_set = self.map.get_mut(k).unwrap_or_else(|| {
            panic!(
                "{}: Cached key from rev_map was not present in forward map!",
                fn_name
            )
        });
        match old_set.len() {
            1 => {
                self.map.remove(k);
            }
            _ => {
                old_set.remove(v);
            }
        }
    }
}

trait HashMapExt<K, V> {
    #[expect(dead_code)]
    fn get_or_insert_default(&mut self, k: &K) -> &V;
    fn get_mut_or_insert_default(&mut self, k: &K) -> &mut V;
}

impl<K: Eq + Hash + Clone, V: Default> HashMapExt<K, V> for HashMap<K, V> {
    fn get_or_insert_default(&mut self, k: &K) -> &V {
        if !self.contains_key(k) {
            self.insert(k.clone(), V::default());
        }
        // We just inserted a value if one wasn't there, so unwrap is ok
        self.get(k).unwrap()
    }

    fn get_mut_or_insert_default(&mut self, k: &K) -> &mut V {
        if !self.contains_key(k) {
            self.insert(k.clone(), V::default());
        }
        // We just inserted a value if one wasn't there, so unwrap is ok
        self.get_mut(k).unwrap()
    }
}

struct MultiMapValueIter<'a, V> {
    inner: Option<Iter<'a, V>>,
}
impl<'a, V> Iterator for MultiMapValueIter<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.as_mut().and_then(|iter| iter.next())
    }
}
