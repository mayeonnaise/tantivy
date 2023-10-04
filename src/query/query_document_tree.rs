use crate::Document;

/// The QueryDocumentTree
/// 
pub enum QueryDocumentTree {
    /// Conjunction
    Conjunction(Vec<QueryDocumentTree>),
    /// Disjunction
    Disjunction(Vec<QueryDocumentTree>),
    /// Term
    Term(),
    /// AnyTerm
    AnyTerm
}

impl QueryDocumentTree {
    fn to_document(&self, document: &Document) {
        
        match self {
            QueryDocumentTree::Conjunction(_) => todo!(),
            QueryDocumentTree::Disjunction(trees) => {
                for tree in trees {
                    tree.to_document(document);
                }
            },
            QueryDocumentTree::Term() => todo!(),
            QueryDocumentTree::AnyTerm => todo!(),
        }
    }
}