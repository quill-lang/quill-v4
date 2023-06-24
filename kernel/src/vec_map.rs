#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VecMap<K, V>(Vec<(K, V)>);
