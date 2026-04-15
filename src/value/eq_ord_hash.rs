use super::*;

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cbor_head()
            .cmp(&other.cbor_head())
            .then_with(|| match (self, other) {
                (Self::TextString(a), Self::TextString(b)) => a.cmp(b),
                (Self::ByteString(a), Self::ByteString(b)) => a.cmp(b),
                (Self::Array(a), Self::Array(b)) => a.cmp(b),
                (Self::Map(a), Self::Map(b)) => a.cmp(b),
                (Self::Tag(_, a), Self::Tag(_, b)) => a.cmp(b),
                _ => std::cmp::Ordering::Equal,
            })
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cbor_head().hash(state);
        match self {
            Self::TextString(s) => s.hash(state),
            Self::ByteString(b) => b.hash(state),
            Self::Array(a) => a.hash(state),
            Self::Map(m) => {
                for (k, v) in m {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Self::Tag(_, v) => v.hash(state),
            _ => {}
        }
    }
}
