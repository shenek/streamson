pub mod simple;

pub use simple::Simple;

pub trait MatchMaker {
    fn match_path(&self, path: &str) -> bool;
}
