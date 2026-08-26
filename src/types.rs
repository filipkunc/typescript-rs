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

/// An equality- and hash-safe JavaScript number used as a literal type key.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NumberLiteral(u64);

impl NumberLiteral {
    /// Construct a numeric literal, canonicalizing negative zero to zero.
    #[must_use]
    pub fn new(value: f64) -> Self {
        let value = if value == 0.0 { 0.0 } else { value };
        Self(value.to_bits())
    }

    /// Recover the numeric value.
    #[must_use]
    pub fn value(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// A named property in an interned object type.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectTypeProperty {
    pub name: String,
    pub type_id: TypeId,
    pub optional: bool,
}

/// The structural key for an interned TypeScript type.
///
/// New compound types are added here as their checker behavior is introduced.
/// Keeping this key structural ensures equivalent types share one compact
/// [`TypeId`].
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
    NumberLiteral(NumberLiteral),
    BigIntLiteral(String),
    StringLiteral(String),
    Union(Box<[TypeId]>),
    Object(Box<[ObjectTypeProperty]>),
    Array(TypeId),
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
            Self::NumberLiteral(value) => return value.value().fmt(formatter),
            Self::BigIntLiteral(value) => return write!(formatter, "{value}n"),
            Self::StringLiteral(value) => return write!(formatter, "\"{value}\""),
            Self::Union(members) => return write!(formatter, "union({} members)", members.len()),
            Self::Object(properties) => {
                return write!(formatter, "object({} properties)", properties.len());
            }
            Self::Array(_) => return formatter.write_str("array"),
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

    /// Construct a normalized union from a sequence of type identities.
    ///
    /// Nested unions are flattened, duplicates and `never` are removed, and
    /// members are sorted so equivalent unions receive the same identity.
    pub fn union(&mut self, members: impl IntoIterator<Item = TypeId>) -> TypeId {
        let primitives = self.primitives();
        let mut stack: Vec<_> = members.into_iter().collect();
        let mut normalized = Vec::with_capacity(stack.len());
        let mut includes_any = false;
        let mut includes_unknown = false;

        while let Some(member) = stack.pop() {
            if member == primitives.any {
                includes_any = true;
            } else if member == primitives.unknown {
                includes_unknown = true;
            } else if member != primitives.never {
                if let Some(TypeKind::Union(nested)) = self.kind(member) {
                    stack.extend(nested.iter().copied());
                } else {
                    normalized.push(member);
                }
            }
        }

        if includes_any {
            return primitives.any;
        }
        if includes_unknown {
            return primitives.unknown;
        }

        normalized.sort_unstable();
        normalized.dedup();
        match normalized.as_slice() {
            [] => primitives.never,
            [only] => *only,
            _ => self.intern(TypeKind::Union(normalized.into_boxed_slice())),
        }
    }

    /// Construct an object type with properties in canonical name order.
    pub fn object(&mut self, properties: impl IntoIterator<Item = ObjectTypeProperty>) -> TypeId {
        let mut properties: Vec<_> = properties.into_iter().collect();
        properties.sort_unstable();
        properties.dedup();
        self.intern(TypeKind::Object(properties.into_boxed_slice()))
    }

    /// Construct an array type with the given element type.
    pub fn array(&mut self, element: TypeId) -> TypeId {
        self.intern(TypeKind::Array(element))
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

    /// Format a type identity, resolving compound members through this store.
    #[must_use]
    pub const fn display(&self, id: TypeId) -> TypeDisplay<'_> {
        TypeDisplay { store: self, id }
    }

    /// Widen a fresh literal type to its primitive counterpart.
    #[must_use]
    pub fn widen_literal(&self, id: TypeId) -> TypeId {
        let primitives = self.primitives();
        match self.kind(id) {
            Some(TypeKind::BooleanLiteral(_)) => primitives.boolean,
            Some(TypeKind::NumberLiteral(_)) => primitives.number,
            Some(TypeKind::BigIntLiteral(_)) => primitives.bigint,
            Some(TypeKind::StringLiteral(_)) => primitives.string,
            _ => id,
        }
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

/// Display adapter for an interned type.
pub struct TypeDisplay<'a> {
    store: &'a TypeStore,
    id: TypeId,
}

impl fmt::Display for TypeDisplay<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(kind) = self.store.kind(self.id) else {
            return formatter.write_str("<invalid type>");
        };
        match kind {
            TypeKind::Union(members) => {
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(" | ")?;
                    }
                    self.store.display(*member).fmt(formatter)?;
                }
                Ok(())
            }
            TypeKind::Object(properties) => {
                if properties.is_empty() {
                    return formatter.write_str("{}");
                }
                formatter.write_str("{ ")?;
                for (index, property) in properties.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str("; ")?;
                    }
                    formatter.write_str(&property.name)?;
                    if property.optional {
                        formatter.write_str("?")?;
                    }
                    formatter.write_str(": ")?;
                    self.store.display(property.type_id).fmt(formatter)?;
                }
                formatter.write_str(" }")
            }
            TypeKind::Array(element) => {
                if matches!(self.store.kind(*element), Some(TypeKind::Union(_))) {
                    formatter.write_str("(")?;
                    self.store.display(*element).fmt(formatter)?;
                    formatter.write_str(")[]")
                } else {
                    self.store.display(*element).fmt(formatter)?;
                    formatter.write_str("[]")
                }
            }
            _ => kind.fmt(formatter),
        }
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
        TypeKind::BooleanLiteral(_)
        | TypeKind::NumberLiteral(_)
        | TypeKind::BigIntLiteral(_)
        | TypeKind::StringLiteral(_)
        | TypeKind::Union(_)
        | TypeKind::Object(_)
        | TypeKind::Array(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{NumberLiteral, ObjectTypeProperty, TypeKind, TypeStore};

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

    #[test]
    fn unions_are_flattened_sorted_and_deduplicated() {
        let mut types = TypeStore::new();
        let string = types.primitives().string;
        let number = types.primitives().number;
        let first = types.union([string, number, string]);
        let second = types.union([number, string]);
        let nested = types.union([first, types.primitives().never]);

        assert_eq!(first, second);
        assert_eq!(first, nested);
        assert_eq!(types.display(first).to_string(), "number | string");
    }

    #[test]
    fn number_literals_canonicalize_negative_zero() {
        assert_eq!(NumberLiteral::new(-0.0), NumberLiteral::new(0.0));
    }

    #[test]
    fn objects_are_canonicalized_by_property_shape() {
        let mut types = TypeStore::new();
        let primitives = types.primitives();
        let first = types.object([
            ObjectTypeProperty {
                name: "name".to_owned(),
                type_id: primitives.string,
                optional: false,
            },
            ObjectTypeProperty {
                name: "id".to_owned(),
                type_id: primitives.number,
                optional: false,
            },
        ]);
        let second = types.object([
            ObjectTypeProperty {
                name: "id".to_owned(),
                type_id: primitives.number,
                optional: false,
            },
            ObjectTypeProperty {
                name: "name".to_owned(),
                type_id: primitives.string,
                optional: false,
            },
        ]);

        assert_eq!(first, second);
        assert_eq!(
            types.display(first).to_string(),
            "{ id: number; name: string }"
        );
    }

    #[test]
    fn arrays_are_canonicalized_by_element_type() {
        let mut types = TypeStore::new();
        let number = types.primitives().number;
        let first = types.array(number);
        let second = types.array(number);
        let nested = types.array(first);

        assert_eq!(first, second);
        assert_eq!(types.display(first).to_string(), "number[]");
        assert_eq!(types.display(nested).to_string(), "number[][]");
    }
}
