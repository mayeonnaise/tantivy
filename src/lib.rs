//#![feature(test,associated_consts)]
#![cfg_attr(test, feature(test))]
#![cfg_attr(test, feature(step_by))]
#![doc(test(attr(allow(unused_variables), deny(warnings))))]


#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate fst;
extern crate byteorder;
extern crate memmap;
extern crate regex;
extern crate tempfile;
extern crate rustc_serialize;
extern crate combine;
extern crate atomicwrites;
extern crate tempdir;
extern crate bincode;
extern crate time;
extern crate libc;
extern crate lz4;
extern crate uuid;
extern crate num_cpus;

#[cfg(test)] extern crate test;
#[cfg(test)] extern crate rand;

mod core;
mod datastruct;
mod postings;
mod directory;
mod compression;
mod fastfield;
mod store;
mod common;
pub mod query;

pub mod analyzer;
pub mod collector;

pub mod schema;

pub use directory::Directory;
pub use core::searcher::Searcher;
pub use core::index::Index;
pub use schema::Term;
pub use schema::Document;
pub use core::reader::SegmentReader;
pub use core::searcher::SegmentLocalId;
pub use self::common::TimerTree;

/// u32 identifying a document within a segment.
/// Document gets their doc id assigned incrementally,
/// as they are added in the segment.
pub type DocId = u32;

#[cfg(test)]
mod tests {

    use super::*;
    use collector::TestCollector;
    use query::MultiTermQuery;

    #[test]
    fn test_indexing() {
        let mut schema = schema::Schema::new();
        let text_field = schema.add_text_field("text", schema::TEXT);

        let index = Index::create_from_tempdir(schema).unwrap();
        {
            // writing the segment
            let mut index_writer = index.writer_with_num_threads(1).unwrap();
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "af b");
                index_writer.add_document(doc).unwrap();
            }
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "a b c");
                index_writer.add_document(doc).unwrap();
            }
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "a b c d");
                index_writer.add_document(doc).unwrap();
            }
            assert!(index_writer.wait().is_ok());
            // TODO reenable this test
            // let segment = commit_result.unwrap();
            // let segment_reader = SegmentReader::open(segment).unwrap();
            // assert_eq!(segment_reader.max_doc(), 3);
        }

    }


    #[test]
    fn test_searcher() {
        let mut schema = schema::Schema::new();
        let text_field = schema.add_text_field("text", schema::TEXT);
        let index = Index::create_in_ram(schema);

        {
            // writing the segment
            let mut index_writer = index.writer_with_num_threads(1).unwrap();
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "af b");
                index_writer.add_document(doc).unwrap();
            }
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "a b c");
                index_writer.add_document(doc).unwrap();
            }
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "a b c d");
                index_writer.add_document(doc).unwrap();
            }
            index_writer.wait().unwrap();
        }
        {
            let searcher = index.searcher().unwrap();
            let get_doc_ids = |terms: Vec<Term>| {
                let query = MultiTermQuery::new(terms);
                let mut collector = TestCollector::new();
                assert!(searcher.search(&query, &mut collector).is_ok());
                collector.docs()
            };
            {
                assert_eq!(
                    get_doc_ids(vec!(Term::from_field_text(&text_field, "a"))),
                    vec!(1, 2));
            }
            {
                assert_eq!(
                    get_doc_ids(vec!(Term::from_field_text(&text_field, "af"))),
                    vec!(0));
            }
            {
                assert_eq!(
                    get_doc_ids(vec!(Term::from_field_text(&text_field, "b"))),
                    vec!(0, 1, 2));
            }
            {
                assert_eq!(
                    get_doc_ids(vec!(Term::from_field_text(&text_field, "c"))),
                    vec!(1, 2));
            }
            {
                assert_eq!(
                    get_doc_ids(vec!(Term::from_field_text(&text_field, "d"))),
                    vec!(2));
            }
            {
                assert_eq!(
                    get_doc_ids(vec!(Term::from_field_text(&text_field, "b"), Term::from_field_text(&text_field, "a"), )),
                    vec!(1, 2));
            }
        }
    }
    
    #[test]
    fn test_searcher_2() {
        let mut schema = schema::Schema::new();
        let text_field = schema.add_text_field("text", schema::TEXT);
        let index = Index::create_in_ram(schema);

        {
            // writing the segment
            let mut index_writer = index.writer_with_num_threads(1).unwrap();
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "af b");
                index_writer.add_document(doc).unwrap();
            }
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "a b c");
                index_writer.add_document(doc).unwrap();
            }
            {
                let mut doc = Document::new();
                doc.add_text(&text_field, "a b c d");
                index_writer.add_document(doc).unwrap();
            }
            index_writer.wait().unwrap();
        }
        index.searcher().unwrap();
    }
}
