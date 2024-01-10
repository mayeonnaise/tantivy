use std::collections::{HashMap, HashSet};

use crate::{schema::Field, TantivyDocument, Term};

use super::{bm25::idf, Bm25StatisticsProvider};

/// The QueryDocumentTree
///
#[derive(Debug)]
pub enum QueryDocumentTree {
    /// Conjunction
    Conjunction(Vec<QueryDocumentTree>),
    /// Disjunction
    Disjunction(Vec<QueryDocumentTree>),
    /// Term
    Term(Term),
    /// AnyTerm
    AnyTerm,
}

impl QueryDocumentTree {
    fn to_document(&self, document: &mut TantivyDocument, scorer: &dyn Bm25StatisticsProvider) {
        let mut field_terms = HashMap::<Field, HashSet<Term>>::new();
        self.to_field_terms(&mut field_terms, scorer);

        for (field, terms) in field_terms.into_iter() {
            let joined_terms = terms
                .into_iter()
                .map(|term| {
                    return match term.clone().value().as_str() {
                        Some(term_value) => term_value.to_string(),
                        None => "".to_string(),
                    };
                })
                .collect::<Vec<String>>()
                .join(" ");

            document.add_text(field, joined_terms);
        }
    }

    fn to_field_terms(
        &self,
        field_terms: &mut HashMap<Field, HashSet<Term>>,
        statistics_provider: &dyn Bm25StatisticsProvider,
    ) {
        match self {
            QueryDocumentTree::Conjunction(trees) => {
                let mut sorted_trees = trees
                    .into_iter()
                    .map(|tree| (tree.score(statistics_provider), tree))
                    .collect::<Vec<(f32, &QueryDocumentTree)>>();

                sorted_trees.sort_by(|(score_a, _), (score_b, _)| {
                    return score_b.total_cmp(score_a);
                });

                if let Some((_, tree_with_highest_score)) = sorted_trees.first() {
                    tree_with_highest_score.to_field_terms(field_terms, statistics_provider);
                }
            }
            QueryDocumentTree::Disjunction(trees) => {
                for tree in trees {
                    tree.to_field_terms(field_terms, statistics_provider);
                }
            }
            QueryDocumentTree::Term(term) => {
                let terms = field_terms
                    .entry(term.field())
                    .or_insert(HashSet::<Term>::new());

                terms.insert(term.clone());
            }
            QueryDocumentTree::AnyTerm => todo!(),
        }
    }

    fn score(&self, statistics_provider: &dyn Bm25StatisticsProvider) -> f32 {
        return match self {
            QueryDocumentTree::Conjunction(trees) => trees.iter().fold(0_f32, |max_score, tree| {
                let tree_score = tree.score(statistics_provider);
                if max_score < tree_score {
                    tree_score
                } else {
                    max_score
                }
            }),
            QueryDocumentTree::Disjunction(trees) => trees.iter().fold(1_f32, |min_score, tree| {
                let tree_score = tree.score(statistics_provider);
                if min_score > tree_score {
                    tree_score
                } else {
                    min_score
                }
            }),
            QueryDocumentTree::Term(term) => {
                return match (
                    statistics_provider.doc_freq(term),
                    statistics_provider.total_num_docs(),
                ) {
                    (Ok(doc_freq), Ok(total_num_docs)) => idf(doc_freq, total_num_docs),
                    _ => 0_f32,
                }
            }
            QueryDocumentTree::AnyTerm => -1_f32,
        };
    }
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, HashSet};

    use crate::{query::Bm25StatisticsProvider, schema::Field, Term};

    use super::QueryDocumentTree;

    struct TestStatisticsProvider {
        document_count: u64,
        term_doc_freq: HashMap<Term, u64>,
    }

    impl Bm25StatisticsProvider for TestStatisticsProvider {
        fn total_num_tokens(&self, _: crate::schema::Field) -> crate::Result<u64> {
            Ok(0)
        }

        fn total_num_docs(&self) -> crate::Result<u64> {
            Ok(self.document_count)
        }

        fn doc_freq(&self, term: &crate::Term) -> crate::Result<u64> {
            Ok(self.term_doc_freq.get(term).map_or(0, |freq| freq.clone()))
        }
    }

    impl TestStatisticsProvider {
        fn add_document(&mut self, document: &str) {
            self.document_count += 1;

            for term in document.split_whitespace() {
                let freq = self
                    .term_doc_freq
                    .entry(Term::from_field_text(Field::from_field_id(0), term))
                    .or_default();
                *freq += 1;
            }
        }
    }

    #[test]
    fn test_term_get_score() {
        // Given
        let mut stats_provider = TestStatisticsProvider {
            document_count: 0,
            term_doc_freq: HashMap::<Term, u64>::new(),
        };
        stats_provider.add_document("This is the first document");
        stats_provider.add_document("This is the second document");
        stats_provider.add_document("This is the third document");

        let document_term = Term::from_field_text(Field::from_field_id(0), "document");
        let document_term_tree = QueryDocumentTree::Term(document_term);
        let first_term = Term::from_field_text(Field::from_field_id(0), "first");
        let first_term_tree = QueryDocumentTree::Term(first_term);
        let non_existent_term = Term::from_field_text(Field::from_field_id(0), "fourth");
        let non_existent_term_tree = QueryDocumentTree::Term(non_existent_term);

        // When
        let document_term_score = document_term_tree.score(&stats_provider);
        let first_term_score = first_term_tree.score(&stats_provider);
        let non_existent_term_score = non_existent_term_tree.score(&stats_provider);

        // Then
        assert_eq!(document_term_score, 0.13353144);
        assert_eq!(first_term_score, 0.9808292);
        assert_eq!(non_existent_term_score, 2.0794415);
    }

    #[test]
    fn test_disjunction_get_score() {
        // Given
        let mut stats_provider = TestStatisticsProvider {
            document_count: 0,
            term_doc_freq: HashMap::<Term, u64>::new(),
        };
        stats_provider.add_document("This is the first document");
        stats_provider.add_document("This is the second document");
        stats_provider.add_document("This is the third document");

        let document_term = Term::from_field_text(Field::from_field_id(0), "document");
        let document_term_tree = QueryDocumentTree::Term(document_term);
        let first_term = Term::from_field_text(Field::from_field_id(0), "first");
        let first_term_tree = QueryDocumentTree::Term(first_term);
        let non_existent_term = Term::from_field_text(Field::from_field_id(0), "fourth");
        let non_existent_term_tree = QueryDocumentTree::Term(non_existent_term);
        let disjunction = QueryDocumentTree::Disjunction(vec![document_term_tree, first_term_tree, non_existent_term_tree]);

        // When
        let disjunction_score = disjunction.score(&stats_provider);

        // Then
        assert_eq!(disjunction_score, 0.13353144);
    }

    #[test]
    fn test_conjunction_get_score() {
        // Given
        let mut stats_provider = TestStatisticsProvider {
            document_count: 0,
            term_doc_freq: HashMap::<Term, u64>::new(),
        };
        stats_provider.add_document("This is the first document");
        stats_provider.add_document("This is the second document");
        stats_provider.add_document("This is the third document");

        let document_term = Term::from_field_text(Field::from_field_id(0), "document");
        let document_term_tree = QueryDocumentTree::Term(document_term);
        let first_term = Term::from_field_text(Field::from_field_id(0), "first");
        let first_term_tree = QueryDocumentTree::Term(first_term);
        let non_existent_term = Term::from_field_text(Field::from_field_id(0), "fourth");
        let non_existent_term_tree = QueryDocumentTree::Term(non_existent_term);
        let conjunction = QueryDocumentTree::Conjunction(vec![document_term_tree, first_term_tree, non_existent_term_tree]);

        // When
        let conjunction_score = conjunction.score(&stats_provider);

        // Then
        assert_eq!(conjunction_score, 2.0794415);
    }

    #[test]
    fn test_term_to_field_terms() {
        // Given
        let mut field_terms = HashMap::<Field, HashSet<Term>>::new();
        
        let mut stats_provider = TestStatisticsProvider {
            document_count: 0,
            term_doc_freq: HashMap::<Term, u64>::new(),
        };
        stats_provider.add_document("This is the first document");

        let document_term = Term::from_field_text(Field::from_field_id(0), "document");
        let document_term_tree = QueryDocumentTree::Term(document_term.clone());

        // When
        document_term_tree.to_field_terms(&mut field_terms, &stats_provider);

        // Then
        let found_field_terms = field_terms.entry(Field::from_field_id(0)).or_default();
        assert!(found_field_terms.contains(&document_term));
    }

    #[test]
    fn test_disjunction_to_field_terms() {
        // Given
        let mut field_terms = HashMap::<Field, HashSet<Term>>::new();
        
        let mut stats_provider = TestStatisticsProvider {
            document_count: 0,
            term_doc_freq: HashMap::<Term, u64>::new(),
        };
        stats_provider.add_document("This is the first document");

        let document_term = Term::from_field_text(Field::from_field_id(0), "document");
        let document_term_tree = QueryDocumentTree::Term(document_term.clone());
        let first_term = Term::from_field_text(Field::from_field_id(0), "first");
        let first_term_tree = QueryDocumentTree::Term(first_term.clone());
        let non_existent_term = Term::from_field_text(Field::from_field_id(0), "fourth");
        let non_existent_term_tree = QueryDocumentTree::Term(non_existent_term.clone());
        let disjunction = QueryDocumentTree::Disjunction(vec![document_term_tree, first_term_tree, non_existent_term_tree]);

        // When
        disjunction.to_field_terms(&mut field_terms, &stats_provider);

        // Then
        let found_field_terms = field_terms.entry(Field::from_field_id(0)).or_default();
        assert!(found_field_terms.contains(&document_term));
        assert!(found_field_terms.contains(&first_term));
        assert!(found_field_terms.contains(&non_existent_term));
    }

    #[test]
    fn test_conjunction_to_field_terms() {
        // Given
        let mut field_terms = HashMap::<Field, HashSet<Term>>::new();
        
        let mut stats_provider = TestStatisticsProvider {
            document_count: 0,
            term_doc_freq: HashMap::<Term, u64>::new(),
        };
        stats_provider.add_document("This is the first document");

        let document_term = Term::from_field_text(Field::from_field_id(0), "document");
        let document_term_tree = QueryDocumentTree::Term(document_term.clone());
        let first_term = Term::from_field_text(Field::from_field_id(0), "first");
        let first_term_tree = QueryDocumentTree::Term(first_term.clone());
        let non_existent_term = Term::from_field_text(Field::from_field_id(0), "fourth");
        let non_existent_term_tree = QueryDocumentTree::Term(non_existent_term.clone());
        let conjunction = QueryDocumentTree::Conjunction(vec![document_term_tree, first_term_tree, non_existent_term_tree]);

        // When
        conjunction.to_field_terms(&mut field_terms, &stats_provider);

        // Then
        let found_field_terms = field_terms.entry(Field::from_field_id(0)).or_default();
        assert!(!found_field_terms.contains(&document_term));
        assert!(!found_field_terms.contains(&first_term));
        assert!(found_field_terms.contains(&non_existent_term));
    }
}
