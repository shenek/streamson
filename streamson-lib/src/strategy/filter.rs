//! The main logic of JSON filtering
//!
//! It uses matchers and filters matched parts
//! from output

use std::{
    collections::VecDeque,
    mem::swap,
    sync::{Arc, Mutex},
};

use crate::{
    error,
    handler::Handler,
    matcher::MatchMaker,
    path::Path,
    streamer::{Streamer, Token},
};

type MatcherItem = (Box<dyn MatchMaker>, Option<Arc<Mutex<dyn Handler>>>);

/// Processes data from input and remove matched parts (and keeps the json valid)
pub struct Filter {
    /// Buffer idx against total idx
    buffer_idx: usize,
    /// Buffer use for input buffering
    buffer: VecDeque<u8>,
    /// Responsible for data extraction
    streamer: Streamer,
    /// Matchers which will cause filtering
    matchers: Vec<MatcherItem>,
    /// What is currently matched - path and indexes to matchers
    matches: Option<(Path, Vec<usize>)>,
    /// Path which data were written to stream for the last time
    last_streaming_path: Option<Path>,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            buffer_idx: 0,
            buffer: VecDeque::new(),
            matchers: vec![],
            streamer: Streamer::new(),
            matches: None,
            last_streaming_path: None,
        }
    }
}

impl Filter {
    /// Create new filter
    ///
    /// It removes matched parts of the input
    pub fn new() -> Self {
        Self::default()
    }

    /// Split working buffer and return the removed part
    ///
    /// # Arguments
    /// * `idx` - total idx to split
    fn move_forward(&mut self, idx: usize) -> VecDeque<u8> {
        let mut splitted = self.buffer.split_off(idx - self.buffer_idx);

        // Swap to return cut part
        swap(&mut self.buffer, &mut splitted);

        self.buffer_idx = idx;

        splitted
    }

    /// Adds new matcher into filtering
    ///
    /// # Arguments
    /// * `matcher` - matcher which matches the path
    /// * `handler` - optinal handler to be used to process data
    ///
    /// # Example
    ///
    /// ```
    /// use streamson_lib::{strategy, matcher};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut filter = strategy::Filter::new();
    /// let matcher = matcher::Simple::new(r#"{"list"}[]"#).unwrap();
    /// filter.add_matcher(
    ///     Box::new(matcher),
    ///     None,
    /// );
    /// ```
    pub fn add_matcher(
        &mut self,
        matcher: Box<dyn MatchMaker>,
        handler: Option<Arc<Mutex<dyn Handler>>>,
    ) {
        self.matchers.push((matcher, handler));
    }

    fn start_handlers(
        &self,
        path: &Path,
        matched_indexes: &[usize],
        token: Token,
    ) -> Result<(), error::General> {
        for (matcher_idx, handler) in matched_indexes
            .iter()
            .filter(|idx| self.matchers[**idx].1.is_some())
            .map(|idx| (idx, self.matchers[*idx].1.as_ref().unwrap()))
        {
            let mut guard = handler.lock().unwrap();
            guard.start(&path, *matcher_idx, token.clone())?;
        }
        Ok(())
    }

    fn feed_handlers(
        &self,
        matched_indexes: &[usize],
        data: VecDeque<u8>,
    ) -> Result<(), error::General> {
        let (first, second) = data.as_slices();
        for (matcher_idx, handler) in matched_indexes
            .iter()
            .filter(|idx| self.matchers[**idx].1.is_some())
            .map(|idx| (idx, self.matchers[*idx].1.as_ref().unwrap()))
        {
            let mut guard = handler.lock().unwrap();
            guard.feed(first, *matcher_idx)?;
            guard.feed(second, *matcher_idx)?;
        }
        Ok(())
    }

    fn end_handlers(
        &self,
        path: &Path,
        matched_indexes: &[usize],
        token: Token,
    ) -> Result<(), error::General> {
        // Trigger handlers start
        for (matcher_idx, handler) in matched_indexes
            .iter()
            .filter(|idx| self.matchers[**idx].1.is_some())
            .map(|idx| (idx, self.matchers[*idx].1.as_ref().unwrap()))
        {
            let mut guard = handler.lock().unwrap();
            guard.end(&path, *matcher_idx, token.clone())?;
        }
        Ok(())
    }

    /// Processes input data
    ///
    /// # Returns
    /// * `Ok(_) processing passed, more data might be needed
    /// * `Err(_)` when input is not correct json
    ///
    /// # Errors
    /// * Error is triggered when incorrect json is detected
    ///   Note that not all json errors are detected
    pub fn process(&mut self, input: &[u8]) -> Result<Vec<u8>, error::General> {
        // Feed the streamer
        self.streamer.feed(input);

        // Feed the input buffer
        self.buffer.extend(input);

        // initialize result
        let mut result = Vec::new();

        // Finish skip

        loop {
            match self.streamer.read()? {
                Token::Start(idx, kind) => {
                    if let Some((path, matched_indexes)) = self.matches.take() {
                        let data = self.move_forward(idx);
                        self.feed_handlers(&matched_indexes, data)?;
                        self.matches = Some((path, matched_indexes));
                    } else {
                        // The path is not matched yet
                        let current_path = self.streamer.current_path().clone();

                        // Try to match current path
                        let matcher_indexes: Vec<usize> = self
                            .matchers
                            .iter()
                            .enumerate()
                            .map(|(idx, matcher)| (idx, matcher.0.match_path(&current_path, kind)))
                            .filter(|(_, matched)| *matched)
                            .map(|(idx, _)| idx)
                            .collect();

                        if !matcher_indexes.is_empty() {
                            // Trigger handlers start
                            self.start_handlers(
                                &current_path,
                                &matcher_indexes,
                                Token::Start(idx, kind),
                            )?;
                            self.matches = Some((current_path, matcher_indexes));
                            self.move_forward(idx); // discard e.g. '"key": '
                        } else {
                            // no match here -> extend output
                            self.last_streaming_path = Some(current_path);
                            result.extend(self.move_forward(idx + 1));
                        }
                    }
                }
                Token::End(idx, kind) => {
                    if let Some((path, matched_indexes)) = self.matches.take() {
                        // Trigger handler feed
                        let data = self.move_forward(idx);
                        self.feed_handlers(&matched_indexes, data)?;

                        if &path == self.streamer.current_path() {
                            // Trigger handlers end
                            self.end_handlers(&path, &matched_indexes, Token::End(idx, kind))?;
                        } else {
                            self.matches = Some((path, matched_indexes));
                        }
                    } else {
                        self.last_streaming_path = Some(self.streamer.current_path().clone());
                        result.extend(self.move_forward(idx));
                    }
                }
                Token::Pending => {
                    return Ok(result);
                }
                Token::Separator(idx) => {
                    if let Some(path) = self.last_streaming_path.as_ref() {
                        if self.streamer.current_path() == path {
                            // removing ',' if the first record from array / object was deleted
                            self.move_forward(idx + 1);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Filter;
    use crate::{
        matcher::{Combinator, Simple},
        test::{Single, Splitter, Window},
    };
    use rstest::*;

    fn get_input() -> Vec<u8> {
        br#"{"users": [{"uid": 1}, {"uid": 2}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
                .to_vec()
    }

    #[test]
    fn single_matcher_no_match() {
        let input = get_input();

        let matcher = Simple::new(r#"{"no-existing"}[]{"uid"}"#).unwrap();
        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(filter.process(&input).unwrap(), input.clone());
    }

    #[test]
    fn single_matcher_array_first() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[0]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{"users": [ {"uid": 2}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_array_last() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[2]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 2}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_array_middle() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[1]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_array_all() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}[]"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{"users": [], "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_object_first() {
        let input = get_input();
        let matcher = Simple::new(r#"{"users"}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{ "groups": [{"gid": 1}, {"gid": 2}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_object_last() {
        let input = get_input();
        let matcher = Simple::new(r#"{"void"}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 2}, {"uid": 3}], "groups": [{"gid": 1}, {"gid": 2}]}"#
        );
    }

    #[test]
    fn single_matcher_object_middle() {
        let input = get_input();
        let matcher = Simple::new(r#"{"groups"}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{"users": [{"uid": 1}, {"uid": 2}, {"uid": 3}], "void": {}}"#
        );
    }

    #[test]
    fn single_matcher_object_all() {
        let input = get_input();
        let matcher = Simple::new(r#"{}"#).unwrap();

        let mut filter = Filter::new();
        filter.add_matcher(Box::new(matcher), None);

        assert_eq!(
            String::from_utf8(filter.process(&input).unwrap()).unwrap(),
            r#"{}"#
        );
    }

    #[rstest(
        splitter,
        case::single(Box::new(Single::new())),
        case::window1(Box::new(Window::new(1))),
        case::window5(Box::new(Window::new(5))),
        case::window100(Box::new(Window::new(100)))
    )]
    fn combinator_slices(splitter: Box<dyn Splitter>) {
        let input = get_input();
        for parts in splitter.split(input) {
            let matcher = Combinator::new(Simple::new(r#"{"users"}"#).unwrap())
                | Combinator::new(Simple::new(r#"{"void"}"#).unwrap());
            let mut filter = Filter::new();
            filter.add_matcher(Box::new(matcher), None);
            let mut result: Vec<u8> = Vec::new();

            for part in parts {
                result.extend(filter.process(&part).unwrap());
            }
            assert_eq!(
                String::from_utf8(result).unwrap(),
                r#"{ "groups": [{"gid": 1}, {"gid": 2}]}"#
            )
        }
    }
}
