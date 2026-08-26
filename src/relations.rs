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
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{TypeKind, TypeStore};

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
}
