use bevy::utils::{HashMap, HashSet};
use std::hash::Hash;

/// Map where a key can have multiple values, but a value can only exist for one key at a time.
/// Re-inserting a value is a no-op if it already exists under the same key, otherwise the value is
/// removed from under it's present key and added under the new key.

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
    pub fn get(&self, k: &K) -> HashSet<V> {
        self.map
            .get(k)
            .map(|set| set.clone())
            .unwrap_or_else(|| HashSet::new())
    }

    /// Returns value's old key
    // Todo: rely a little less on clone
    pub fn insert(&mut self, new_k: &K, v: &V) -> Option<K> {
        let maybe_old_k = self.rev_map.insert(v.clone(), new_k.clone());

        if let Some(old_k) = &maybe_old_k {
            // insert value into same key: no-op
            if old_k == new_k {
                return maybe_old_k;
            }

            // remove old value; its key must exist according to rev_map
            let old_set = self.map.get_mut(&old_k).unwrap();
            match old_set.len() {
                1 => {
                    self.map.remove(old_k);
                }
                _ => {
                    old_set.remove(v);
                }
            }
        }
        // insert new value
        self.map.get_mut_or_insert_default(new_k).insert(v.clone());

        maybe_old_k
    }
}

trait HashMapExt<K, V> {
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
