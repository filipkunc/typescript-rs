use std::collections::HashMap;

use crate::types::{TypeId, TypeKind, TypeStore};

#[derive(Clone, Copy)]
enum RelationState {
    InProgress,
    Complete(bool),
}

/// Cached structural relations for one checker run.
#[derive(Default)]
pub(crate) struct TypeRelations {
    assignable: HashMap<(TypeId, TypeId), RelationState>,
}

impl TypeRelations {
    pub(crate) fn is_assignable(
        &mut self,
        types: &TypeStore,
        source: TypeId,
        target: TypeId,
    ) -> bool {
        if source == target {
            return true;
        }
        match self.assignable.get(&(source, target)) {
            Some(RelationState::Complete(result)) => return *result,
            // Recursive structural types use an optimistic assumption. A
            // later object-type implementation can revisit this edge if a
            // property comparison disproves the relation.
            Some(RelationState::InProgress) => return true,
            None => {}
        }

        self.assignable
            .insert((source, target), RelationState::InProgress);
        let result = self.compute_assignable(types, source, target);
        self.assignable
            .insert((source, target), RelationState::Complete(result));
        result
    }

    fn compute_assignable(&mut self, types: &TypeStore, source: TypeId, target: TypeId) -> bool {
        let primitives = types.primitives();
        if source == primitives.any || source == primitives.never {
            return true;
        }
        if target == primitives.any || target == primitives.unknown {
            return true;
        }
        if source == primitives.undefined && target == primitives.void {
            return true;
        }

        match (types.kind(source), types.kind(target)) {
            (Some(TypeKind::Union(sources)), _) => sources
                .iter()
                .all(|source| self.is_assignable(types, *source, target)),
            (_, Some(TypeKind::Union(targets))) => targets
                .iter()
                .any(|target| self.is_assignable(types, source, *target)),
            (Some(TypeKind::BooleanLiteral(_)), Some(TypeKind::Boolean))
            | (Some(TypeKind::NumberLiteral(_)), Some(TypeKind::Number))
            | (Some(TypeKind::BigIntLiteral(_)), Some(TypeKind::BigInt))
            | (Some(TypeKind::StringLiteral(_)), Some(TypeKind::String)) => true,
            (Some(TypeKind::Object(source)), Some(TypeKind::Object(target))) => {
                target.iter().all(|target_property| {
                    let source_property = source
                        .iter()
                        .find(|property| property.name == target_property.name);
                    match source_property {
                        Some(source_property) => {
                            (target_property.optional || !source_property.optional)
                                && self.is_assignable(
                                    types,
                                    source_property.type_id,
                                    target_property.type_id,
                                )
                        }
                        None => target_property.optional,
                    }
                })
            }
            (Some(TypeKind::Array(source)), Some(TypeKind::Array(target))) => {
                self.is_assignable(types, *source, *target)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{ObjectTypeProperty, TypeKind, TypeStore};

    use super::TypeRelations;

    #[test]
    fn literals_are_assignable_to_primitives_and_matching_unions() {
        let mut types = TypeStore::new();
        let open = types.intern(TypeKind::StringLiteral("open".to_owned()));
        let closed = types.intern(TypeKind::StringLiteral("closed".to_owned()));
        let pending = types.intern(TypeKind::StringLiteral("pending".to_owned()));
        let status = types.union([open, closed]);
        let string = types.primitives().string;
        let mut relations = TypeRelations::default();

        assert!(relations.is_assignable(&types, open, status));
        assert!(relations.is_assignable(&types, open, string));
        assert!(!relations.is_assignable(&types, pending, status));
    }

    #[test]
    fn objects_are_assignable_by_required_property_shape() {
        let mut types = TypeStore::new();
        let primitives = types.primitives();
        let target = types.object([
            ObjectTypeProperty {
                name: "id".to_owned(),
                type_id: primitives.number,
                optional: false,
            },
            ObjectTypeProperty {
                name: "name".to_owned(),
                type_id: primitives.string,
                optional: true,
            },
        ]);
        let valid = types.object([ObjectTypeProperty {
            name: "id".to_owned(),
            type_id: primitives.number,
            optional: false,
        }]);
        let missing = types.object([ObjectTypeProperty {
            name: "name".to_owned(),
            type_id: primitives.string,
            optional: false,
        }]);
        let wrong = types.object([ObjectTypeProperty {
            name: "id".to_owned(),
            type_id: primitives.string,
            optional: false,
        }]);
        let mut relations = TypeRelations::default();

        assert!(relations.is_assignable(&types, valid, target));
        assert!(!relations.is_assignable(&types, missing, target));
        assert!(!relations.is_assignable(&types, wrong, target));
    }

    #[test]
    fn arrays_are_assignable_by_element_type() {
        let mut types = TypeStore::new();
        let primitives = types.primitives();
        let numbers = types.array(primitives.number);
        let strings = types.array(primitives.string);
        let empty = types.array(primitives.never);
        let mut relations = TypeRelations::default();

        assert!(relations.is_assignable(&types, numbers, numbers));
        assert!(relations.is_assignable(&types, empty, numbers));
        assert!(!relations.is_assignable(&types, strings, numbers));
    }

    #[test]
    fn undefined_is_assignable_to_void() {
        let types = TypeStore::new();
        let primitives = types.primitives();
        let mut relations = TypeRelations::default();

        assert!(relations.is_assignable(&types, primitives.undefined, primitives.void));
    }
}
