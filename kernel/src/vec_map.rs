#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VecMap<K, V>(Vec<(K, V)>);

impl<K, V> From<Vec<(K, V)>> for VecMap<K, V> {
    fn from(value: Vec<(K, V)>) -> Self {
        Self(value)
    }
}

impl<K, V> VecMap<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn into_iter(self) -> impl Iterator<Item = (K, V)> {
        self.0.into_iter()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(K, V)> {
        self.0.iter()
    }
}
