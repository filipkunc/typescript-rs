//! Interned representations of TypeScript types.

use std::{collections::HashMap, fmt};

/// A compact identity for a type interned in a [`TypeStore`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeId(u32);

impl TypeId {
    /// The zero-based store index represented by this identity.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// The structural key for an interned TypeScript type.
///
/// Compound and literal types will be added here as their checker behavior is
/// introduced. Keeping this key structural ensures equivalent types share one
/// compact [`TypeId`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TypeKind {
    Any,
    Unknown,
    Never,
    Void,
    Undefined,
    Null,
    Boolean,
    Number,
    BigInt,
    String,
    BooleanLiteral(bool),
    StringLiteral(String),
}

impl fmt::Display for TypeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Any => "any",
            Self::Unknown => "unknown",
            Self::Never => "never",
            Self::Void => "void",
            Self::Undefined => "undefined",
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::BigInt => "bigint",
            Self::String => "string",
            Self::BooleanLiteral(value) => return value.fmt(formatter),
            Self::StringLiteral(value) => return write!(formatter, "\"{value}\""),
        };
        formatter.write_str(name)
    }
}

/// Canonical identities for the built-in primitive types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimitiveTypes {
    pub any: TypeId,
    pub unknown: TypeId,
    pub never: TypeId,
    pub void: TypeId,
    pub undefined: TypeId,
    pub null: TypeId,
    pub boolean: TypeId,
    pub number: TypeId,
    pub bigint: TypeId,
    pub string: TypeId,
}

const PRIMITIVE_TYPES: PrimitiveTypes = PrimitiveTypes {
    any: TypeId(0),
    unknown: TypeId(1),
    never: TypeId(2),
    void: TypeId(3),
    undefined: TypeId(4),
    null: TypeId(5),
    boolean: TypeId(6),
    number: TypeId(7),
    bigint: TypeId(8),
    string: TypeId(9),
};

const PRIMITIVE_KINDS: [TypeKind; 10] = [
    TypeKind::Any,
    TypeKind::Unknown,
    TypeKind::Never,
    TypeKind::Void,
    TypeKind::Undefined,
    TypeKind::Null,
    TypeKind::Boolean,
    TypeKind::Number,
    TypeKind::BigInt,
    TypeKind::String,
];

/// Owns and deduplicates all types for one checker program generation.
#[derive(Debug)]
pub struct TypeStore {
    /// Non-primitive types. Primitive identities occupy fixed slots and do
    /// not require heap allocation or hash-table entries.
    kinds: Vec<TypeKind>,
    interned: HashMap<TypeKind, TypeId>,
}

impl TypeStore {
    /// Construct a store with stable canonical identities for primitive types.
    #[must_use]
    pub fn new() -> Self {
        Self {
            kinds: Vec::new(),
            interned: HashMap::new(),
        }
    }

    /// Return the canonical primitive type identities.
    #[must_use]
    pub const fn primitives(&self) -> PrimitiveTypes {
        PRIMITIVE_TYPES
    }

    /// Intern a structural type key and return its canonical identity.
    pub fn intern(&mut self, kind: TypeKind) -> TypeId {
        if let Some(id) = primitive_id(&kind) {
            return id;
        }
        if let Some(id) = self.interned.get(&kind) {
            return *id;
        }
        insert_new(&mut self.kinds, &mut self.interned, kind)
    }

    /// Resolve a type identity to its structural key.
    #[must_use]
    pub fn kind(&self, id: TypeId) -> Option<&TypeKind> {
        let index = usize::try_from(id.0).ok()?;
        if index < PRIMITIVE_KINDS.len() {
            return PRIMITIVE_KINDS.get(index);
        }
        self.kinds.get(index - PRIMITIVE_KINDS.len())
    }

    /// Number of unique types currently interned.
    #[must_use]
    pub fn len(&self) -> usize {
        PRIMITIVE_KINDS.len() + self.kinds.len()
    }

    /// Whether no types have been interned.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

impl Default for TypeStore {
    fn default() -> Self {
        Self::new()
    }
}

fn insert_new(
    kinds: &mut Vec<TypeKind>,
    interned: &mut HashMap<TypeKind, TypeId>,
    kind: TypeKind,
) -> TypeId {
    let compound_index = u32::try_from(kinds.len()).expect("type store exceeded u32::MAX entries");
    let id = TypeId(
        u32::try_from(PRIMITIVE_KINDS.len())
            .expect("primitive type count must fit in u32")
            .checked_add(compound_index)
            .expect("type store exceeded u32::MAX entries"),
    );
    kinds.push(kind.clone());
    interned.insert(kind, id);
    id
}

const fn primitive_id(kind: &TypeKind) -> Option<TypeId> {
    match kind {
        TypeKind::Any => Some(PRIMITIVE_TYPES.any),
        TypeKind::Unknown => Some(PRIMITIVE_TYPES.unknown),
        TypeKind::Never => Some(PRIMITIVE_TYPES.never),
        TypeKind::Void => Some(PRIMITIVE_TYPES.void),
        TypeKind::Undefined => Some(PRIMITIVE_TYPES.undefined),
        TypeKind::Null => Some(PRIMITIVE_TYPES.null),
        TypeKind::Boolean => Some(PRIMITIVE_TYPES.boolean),
        TypeKind::Number => Some(PRIMITIVE_TYPES.number),
        TypeKind::BigInt => Some(PRIMITIVE_TYPES.bigint),
        TypeKind::String => Some(PRIMITIVE_TYPES.string),
        TypeKind::BooleanLiteral(_) | TypeKind::StringLiteral(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{TypeKind, TypeStore};

    #[test]
    fn primitives_have_canonical_ids() {
        let mut types = TypeStore::new();
        let primitives = types.primitives();

        assert_eq!(types.intern(TypeKind::String), primitives.string);
        assert_eq!(types.kind(primitives.number), Some(&TypeKind::Number));
        assert_eq!(types.len(), 10);
    }

    #[test]
    fn structural_types_are_deduplicated() {
        let mut types = TypeStore::new();
        let first = types.intern(TypeKind::StringLiteral("tsrs".to_owned()));
        let second = types.intern(TypeKind::StringLiteral("tsrs".to_owned()));

        assert_eq!(first, second);
        assert_eq!(types.len(), 11);
        assert_eq!(
            types.kind(first),
            Some(&TypeKind::StringLiteral("tsrs".to_owned()))
        );
    }
}
