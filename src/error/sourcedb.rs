use core::slice;
use std::{
    cell::UnsafeCell,
    collections::HashMap,
    ffi::{OsStr, OsString},
    str,
};

use codespan_reporting::files::Files;

use crate::util::DisplayableOsStr;

mod util;

struct Source {
    src: util::RawString,
    /// The starting byte index of each line
    line_starts: util::RawVec<usize>,
}

pub struct SourceDB {
    filename_to_index: UnsafeCell<HashMap<util::RawOsString, usize>>,
    index_to_filename: UnsafeCell<Vec<util::RawOsStr>>,
    sources: UnsafeCell<Vec<Source>>,
}

impl Default for SourceDB {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceDB {
    pub fn new() -> Self {
        Self {
            filename_to_index: UnsafeCell::new(HashMap::new()),
            index_to_filename: UnsafeCell::new(Vec::new()),
            sources: UnsafeCell::new(Vec::new()),
        }
    }

    pub fn index(&self, path: &OsStr) -> Option<usize> {
        unsafe { &*self.filename_to_index.get() }.get(path).copied()
    }

    pub fn name(&self, index: usize) -> Option<&OsStr> {
        let name_to_index = unsafe { &*self.index_to_filename.get() };

        let raw_name = name_to_index.get(index)?;
        let name = unsafe {
            OsStr::from_encoded_bytes_unchecked(slice::from_raw_parts(raw_name.data, raw_name.len))
        };

        Some(name)
    }

    pub fn source(&self, index: usize) -> Option<(&str, &[usize])> {
        let sources = unsafe { &*self.sources.get() };

        sources
            .get(index)
            .map(|Source { src, line_starts }| unsafe {
                (
                    str::from_utf8_unchecked(slice::from_raw_parts(src.ptr, src.len)),
                    slice::from_raw_parts(line_starts.ptr, line_starts.len),
                )
            })
    }

    pub fn add(&self, path: OsString, src: String) -> usize {
        let sources = unsafe { &mut *self.sources.get() };
        let filename_to_index = unsafe { &mut *self.filename_to_index.get() };
        let index_to_filename = unsafe { &mut *self.index_to_filename.get() };

        let index = sources.len();

        if filename_to_index.get(&*path).is_some() {
            panic!("Path {path:?} is already in SourceDB");
        }

        let path = util::RawOsString::from(path);

        index_to_filename.push(util::RawOsStr {
            data: path.0.ptr,
            len: path.0.len,
        });
        filename_to_index.insert(path, index);

        let line_starts: Vec<usize> = codespan_reporting::files::line_starts(&src).collect();

        sources.push(Source {
            src: src.into(),
            line_starts: line_starts.into(),
        });

        index
    }
}

impl<'a> Files<'a> for SourceDB {
    type FileId = usize;

    type Name = DisplayableOsStr<'a>;

    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        self.name(id)
            .map(DisplayableOsStr)
            .ok_or(codespan_reporting::files::Error::FileMissing)
    }

    fn source(
        &'a self,
        id: Self::FileId,
    ) -> Result<Self::Source, codespan_reporting::files::Error> {
        self.source(id)
            .map(|(src, _)| src)
            .ok_or(codespan_reporting::files::Error::FileMissing)
    }

    fn line_index(
        &'a self,
        id: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        let Some((_, line_starts)) = self.source(id) else {
            return Err(codespan_reporting::files::Error::FileMissing);
        };

        let line_idx = line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|e| e - 1);

        Ok(line_idx)
    }

    fn line_range(
        &'a self,
        id: Self::FileId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
        let Some((src, line_starts)) = self.source(id) else {
            return Err(codespan_reporting::files::Error::FileMissing);
        };

        let Some(&start) = line_starts.get(line_index) else {
            return Err(codespan_reporting::files::Error::LineTooLarge {
                given: line_index,
                max: line_starts.len() - 1,
            });
        };

        let end = line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(src.len());

        Ok(start..end)
    }
}
