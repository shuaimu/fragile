use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Unique identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

impl SourceId {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// The language of a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Cpp,
    Go,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "h" => Some(Language::Cpp),
            "go" => Some(Language::Go),
            _ => None,
        }
    }
}

/// A source file with its contents.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: PathBuf,
    pub content: String,
    pub language: Language,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(id: SourceId, path: PathBuf, content: String, language: Language) -> Self {
        let line_starts = std::iter::once(0)
            .chain(content.match_indices('\n').map(|(i, _)| i as u32 + 1))
            .collect();

        Self {
            id,
            path,
            content,
            language,
            line_starts,
        }
    }

    /// Get line and column (0-indexed) from byte offset.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let line = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let col = offset - self.line_starts[line];
        (line as u32, col)
    }

    /// Get the content of a specific line.
    pub fn line(&self, line: u32) -> &str {
        let start = self.line_starts[line as usize] as usize;
        let end = self
            .line_starts
            .get(line as usize + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.content.len());
        &self.content[start..end].trim_end_matches('\n')
    }
}

/// Registry of all source files.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: RwLock<Vec<SourceFile>>,
    path_to_id: RwLock<FxHashMap<PathBuf, SourceId>>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&self, path: impl AsRef<Path>, content: String) -> miette::Result<SourceId> {
        let path = path.as_ref().to_path_buf();

        let language = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(Language::from_extension)
            .ok_or_else(|| miette::miette!("Unknown file extension: {:?}", path))?;

        let mut files = self.files.write().unwrap();
        let mut path_to_id = self.path_to_id.write().unwrap();

        let id = SourceId(files.len() as u32);
        let file = SourceFile::new(id, path.clone(), content, language);
        files.push(file);
        path_to_id.insert(path, id);

        Ok(id)
    }

    pub fn get(&self, id: SourceId) -> Option<SourceFile> {
        let files = self.files.read().unwrap();
        files.get(id.0 as usize).cloned()
    }

    pub fn get_by_path(&self, path: impl AsRef<Path>) -> Option<SourceFile> {
        let path_to_id = self.path_to_id.read().unwrap();
        let id = path_to_id.get(path.as_ref())?;
        self.get(*id)
    }
}
