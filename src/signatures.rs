use crate::types::TypeId;

/// A compact identity for a callable signature owned by one checker run.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SignatureId(u32);

#[derive(Debug)]
pub(crate) struct SignatureParameter {
    pub(crate) type_id: TypeId,
    pub(crate) diagnostic_name: String,
}

/// The explicitly annotated shape of a supported callable declaration or expression.
#[derive(Debug)]
pub(crate) struct Signature {
    pub(crate) parameters: Box<[SignatureParameter]>,
    pub(crate) return_type: TypeId,
    pub(crate) return_diagnostic_name: String,
}

/// Signatures keep diagnostic metadata separate from the canonical
/// `TypeKind::Function` identity used by structural callable shapes.
#[derive(Debug, Default)]
pub(crate) struct SignatureStore {
    signatures: Vec<Signature>,
}

impl SignatureStore {
    pub(crate) fn add(&mut self, signature: Signature) -> SignatureId {
        let index = u32::try_from(self.signatures.len())
            .expect("signature store exceeded u32::MAX entries");
        self.signatures.push(signature);
        SignatureId(index)
    }

    pub(crate) fn get(&self, id: SignatureId) -> Option<&Signature> {
        self.signatures.get(usize::try_from(id.0).ok()?)
    }
}
