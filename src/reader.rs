//! Contains high-level interface for a pull-based XML parser.
//!
//! The most important type in this module is `EventReader`, which provides an iterator
//! view for events in XML document.

use core::iter::{Copied, FusedIterator};
use core::result;

use crate::common::{Position, TextPosition};

pub use self::config::ParserConfig;
pub use self::config::ParserConfig2;
pub use self::error::{Error, ErrorKind};
pub use self::events::XmlEvent;

use self::parser::PullParser;

mod config;
mod events;
mod lexer;
mod parser;
mod error;


/// A result type yielded by `XmlReader`.
pub type Result<T, E = Error> = result::Result<T, E>;

/// A wrapper around an Iterator which provides pull-based XML parsing.
pub struct EventReader<S: Iterator<Item=u8>> {
    source: S,
    parser: PullParser,
}

impl<S: Iterator<Item=u8>> EventReader<S> {
    /// Creates a new reader from an Iterator.
    #[inline]
    pub fn new(source: S) -> EventReader<S> {
        EventReader::new_with_config(source, ParserConfig2::new())
    }

    /// Creates a new reader with the provided configuration from an Iterator.
    #[inline]
    pub fn new_with_config(source: S, config: impl Into<ParserConfig2>) -> EventReader<S> {
        EventReader { source, parser: PullParser::new(config) }
    }

    /// Pulls and returns next XML event from the Iterator.
    ///
    /// If returned event is `XmlEvent::Error` or `XmlEvent::EndDocument`, then
    /// further calls to this method will return this event again.
    #[inline]
    pub fn next(&mut self) -> Result<XmlEvent> {
        self.parser.next(&mut self.source)
    }

    /// Skips all XML events until the next end tag at the current level.
    ///
    /// Convenience function that is useful for the case where you have
    /// encountered a start tag that is of no interest and want to
    /// skip the entire XML subtree until the corresponding end tag.
    #[inline]
    pub fn skip(&mut self) -> Result<()> {
        let mut depth = 1;

        while depth > 0 {
            match self.next()? {
                XmlEvent::StartElement { .. } => depth += 1,
                XmlEvent::EndElement { .. } => depth -= 1,
                XmlEvent::EndDocument => unreachable!(),
                _ => {}
            }
        }

        Ok(())
    }

    pub fn source(&self) -> &S { &self.source }
    pub fn source_mut(&mut self) -> &mut S { &mut self.source }

    /// Unwraps this `EventReader`, returning the underlying Iterator.
    pub fn into_inner(self) -> S {
        self.source
    }
}

impl<S: Iterator<Item=u8>> Position for EventReader<S> {
    /// Returns the position of the last event produced by the reader.
    #[inline]
    fn position(&self) -> TextPosition {
        self.parser.position()
    }
}

impl<S: Iterator<Item=u8>> IntoIterator for EventReader<S> {
    type Item = Result<XmlEvent>;
    type IntoIter = Events<S>;

    fn into_iter(self) -> Events<S> {
        Events { reader: self, finished: false }
    }
}

/// An iterator over XML events created from some type implementing `Iterator<Item = &u8>`.
///
/// When the next event is `xml::event::Error` or `xml::event::EndDocument`, then
/// it will be returned by the iterator once, and then it will stop producing events.
pub struct Events<S: Iterator<Item=u8>> {
    reader: EventReader<S>,
    finished: bool,
}

impl<S: Iterator<Item=u8>> Events<S> {
    /// Unwraps the iterator, returning the internal `EventReader`.
    #[inline]
    pub fn into_inner(self) -> EventReader<S> {
        self.reader
    }

    pub fn source(&self) -> &S { &self.reader.source }
    pub fn source_mut(&mut self) -> &mut S { &mut self.reader.source }

}

impl<S: Iterator<Item=u8>> FusedIterator for Events<S> {
}

impl<S: Iterator<Item=u8>> Iterator for Events<S> {
    type Item = Result<XmlEvent>;

    #[inline]
    fn next(&mut self) -> Option<Result<XmlEvent>> {
        if self.finished && !self.reader.parser.is_ignoring_end_of_stream() {
            None
        } else {
            let ev = self.reader.next();
            if let Ok(XmlEvent::EndDocument) | Err(_) = ev {
                self.finished = true;
            }
            Some(ev)
        }
    }
}

impl<'a> EventReader<Copied<core::slice::Iter<'a, u8>>> {
    /// A convenience method to create an `XmlReader` from a string slice.
    #[inline]
    #[must_use]
    pub fn from_str(source: &'a str) -> EventReader<Copied<core::slice::Iter<'a, u8>>> {
        EventReader::new(source.as_bytes().into_iter().copied())
    }
}
