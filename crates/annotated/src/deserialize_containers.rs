use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::{self, Display};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

use serde::Deserializer;
use serde::de::{MapAccess, SeqAccess, Visitor};

use crate::deserialize::DeserializeAt;
use crate::{Deserialize, FoundIssues, Location};

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Option<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_option(VisitorAt::<Self>::new(at, found))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Box<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        let value = T::deserialize_at(deserializer, at, found)?;
        Ok(value.map(Box::new))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Box<[T]> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        let elements = Vec::<T>::deserialize_at(deserializer, at, found)?;
        Ok(elements.map(Vec::into_boxed_slice))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vec<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_seq(VisitorAt::<Self>::new(at, found))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for VecDeque<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_seq(VisitorAt::<Self>::new(at, found))
    }
}

impl<'de, K, V> Deserialize<'de> for BTreeMap<K, V>
where
    K: serde::Deserialize<'de> + Display + Ord,
    V: Deserialize<'de>,
{
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_map(VisitorAt::<Self>::new(at, found))
    }
}

/// A key is cloned when its value was built but has issues, so that they can be reported in the order the map
/// iterates over its values.
impl<'de, K, V, S> Deserialize<'de> for HashMap<K, V, S>
where
    K: serde::Deserialize<'de> + Display + Eq + Hash + Clone,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut FoundIssues,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_map(VisitorAt::<Self>::new(at, found))
    }
}

struct VisitorAt<'a, 'l, C> {
    at: &'a Location<'l>,
    found: &'a mut FoundIssues,
    container: PhantomData<fn() -> C>,
}

impl<'a, 'l, C> VisitorAt<'a, 'l, C> {
    fn new(at: &'a Location<'l>, found: &'a mut FoundIssues) -> Self {
        Self {
            at,
            found,
            container: PhantomData,
        }
    }

    fn read_elements<'de, A, T>(
        self,
        mut seq: A,
        mut elements: C,
        mut push: impl FnMut(&mut C, T),
    ) -> Result<Option<C>, A::Error>
    where
        A: SeqAccess<'de>,
        T: Deserialize<'de>,
    {
        let mut index = 0;
        let mut is_every_element_built = true;
        while let Some(element) = seq.next_element_seed(DeserializeAt::<T>::new(&self.at.index(index), self.found))? {
            match element {
                Some(element) => push(&mut elements, element),
                None => is_every_element_built = false,
            }
            index += 1;
        }
        Ok(is_every_element_built.then_some(elements))
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for VisitorAt<'_, '_, Option<T>> {
    type Value = Option<Option<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("option")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Some(None))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Some(None))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let value = T::deserialize_at(deserializer, self.at, self.found)?;
        Ok(value.map(Some))
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for VisitorAt<'_, '_, Vec<T>> {
    type Value = Option<Vec<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let elements = Vec::with_capacity(room_for::<T>(seq.size_hint()));
        self.read_elements(seq, elements, Vec::push)
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for VisitorAt<'_, '_, VecDeque<T>> {
    type Value = Option<VecDeque<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let elements = VecDeque::with_capacity(room_for::<T>(seq.size_hint()));
        self.read_elements(seq, elements, VecDeque::push_back)
    }
}

impl<'de, K, V> Visitor<'de> for VisitorAt<'_, '_, BTreeMap<K, V>>
where
    K: serde::Deserialize<'de> + Display + Ord,
    V: Deserialize<'de>,
{
    type Value = Option<BTreeMap<K, V>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = BTreeMap::new();
        let mut values_with_issues = BTreeMap::new();
        while let Some(key) = map.next_key::<K>()? {
            let issues_before = self.found.len();
            let value = map.next_value_seed(DeserializeAt::<V>::new(&self.at.key(&key), self.found))?;
            let has_issues = self.found.len() > issues_before;
            match value {
                Some(value) if !has_issues => {
                    values.insert(key, value);
                }
                value => {
                    let found_in_value = self.found.take_those_after(issues_before);
                    values.remove(&key);
                    values_with_issues.insert(key, (value, found_in_value));
                }
            }
        }
        let mut is_every_value_built = true;
        for (key, (value, mut found_in_value)) in values_with_issues {
            let is_given_again_without_issues = values.contains_key(&key);
            if is_given_again_without_issues {
                continue;
            }
            self.found.append(&mut found_in_value);
            match value {
                Some(value) => {
                    values.insert(key, value);
                }
                None => is_every_value_built = false,
            }
        }
        Ok(is_every_value_built.then_some(values))
    }
}

impl<'de, K, V, S> Visitor<'de> for VisitorAt<'_, '_, HashMap<K, V, S>>
where
    K: serde::Deserialize<'de> + Display + Eq + Hash + Clone,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    type Value = Option<HashMap<K, V, S>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = HashMap::with_capacity_and_hasher(room_for::<(K, V)>(map.size_hint()), S::default());
        let mut issues_by_key = IssuesByKey::<K, S>::none();
        while let Some(key) = map.next_key::<K>()? {
            let mut found_in_value = self.found.new_alike();
            let value = map.next_value_seed(DeserializeAt::<V>::new(&self.at.key(&key), &mut found_in_value))?;
            match value {
                Some(value) if found_in_value.is_empty() => {
                    issues_by_key.forget(&key);
                    values.insert(key, value);
                }
                Some(value) => {
                    issues_by_key.keep(key.clone(), found_in_value);
                    values.insert(key, value);
                }
                None => {
                    values.remove(&key);
                    issues_by_key.keep(key, found_in_value);
                }
            }
        }
        let is_every_value_built = issues_by_key.has_only_keys_of(&values);
        issues_by_key.join_in_the_order_of(&values, self.found);
        Ok(is_every_value_built.then_some(values))
    }
}

struct IssuesByKey<K, S> {
    issues_by_key: Option<HashMap<K, FoundIssues, S>>,
}

impl<K: Eq + Hash, S: BuildHasher + Default> IssuesByKey<K, S> {
    const fn none() -> Self {
        Self { issues_by_key: None }
    }

    fn keep(&mut self, key: K, found_in_value: FoundIssues) {
        self.issues_by_key
            .get_or_insert_with(|| HashMap::with_hasher(S::default()))
            .insert(key, found_in_value);
    }

    fn forget(&mut self, key: &K) {
        if let Some(issues_by_key) = &mut self.issues_by_key {
            issues_by_key.remove(key);
        }
    }

    fn has_only_keys_of<V>(&self, values: &HashMap<K, V, S>) -> bool {
        self.issues_by_key
            .iter()
            .flat_map(HashMap::keys)
            .all(|key| values.contains_key(key))
    }

    fn join_in_the_order_of<V>(self, values: &HashMap<K, V, S>, found: &mut FoundIssues) {
        let Some(mut issues_by_key) = self.issues_by_key else {
            return;
        };
        for key in values.keys() {
            if let Some(mut found_in_value) = issues_by_key.remove(key) {
                found.append(&mut found_in_value);
            }
        }
        for mut found_in_value in issues_by_key.into_values() {
            found.append(&mut found_in_value);
        }
    }
}

// serde's own cap on the room a size hint reserves: the hint is the input's word, so trusting it whole would let
// a short input ask for any amount of memory.
pub(crate) fn room_for<Element>(size_hint: Option<usize>) -> usize {
    const MOST_BYTES_RESERVED: usize = 1024 * 1024;
    match size_of::<Element>() {
        0 => 0,
        element_size => size_hint.unwrap_or(0).min(MOST_BYTES_RESERVED / element_size),
    }
}
