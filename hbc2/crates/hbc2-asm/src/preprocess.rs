//! Loads the entry file and everything it includes, and applies `.define`s.
//!
//! * `.include "file.has"`: the path is relative to the including file. A file
//!   is included the first time only; later includes of it are skipped.
//! * `.define NAME value…`: global. Every `NAME` word in the project is
//!   replaced by the value's tokens. Defines may use other defines.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::diagnostic::{Loc, Report};
use crate::lexer::{Tok, Token, lex_line};
use crate::parser::is_reserved;
use crate::source::{SourceProvider, normalize, same_file_key};

/// One non-empty source line, after `.include` / `.define` handling.
pub(crate) struct Line {
    pub file: usize,
    pub line: u32,
    pub tokens: Vec<Token>,
}

pub(crate) enum Event {
    Line(Line),
    /// Entering or leaving an included file: the current code block ends.
    BlockBreak,
}

pub(crate) struct Loaded {
    pub files: Vec<PathBuf>,
    pub events: Vec<Event>,
}

struct Define {
    tokens: Vec<Token>,
    loc: Loc,
}

const MAX_DEFINE_DEPTH: usize = 16;

pub(crate) fn load(entry: &Path, provider: &dyn SourceProvider, report: &mut Report) -> Loaded {
    let mut loader = Loader {
        provider,
        report,
        files: Vec::new(),
        seen: HashSet::new(),
        events: Vec::new(),
        defines: HashMap::new(),
    };
    loader.load_file(entry.to_path_buf(), None);

    let Loader {
        files,
        events,
        defines,
        report,
        ..
    } = loader;
    let events = events
        .into_iter()
        .map(|event| match event {
            Event::Line(mut line) => {
                let loc = Loc::new(line.file, line.line, 0);
                match expand(&line.tokens, &defines, 0) {
                    Ok(tokens) => line.tokens = tokens,
                    Err(message) => report.error(loc, message),
                }
                Event::Line(line)
            }
            other => other,
        })
        .collect();
    Loaded { files, events }
}

struct Loader<'a> {
    provider: &'a dyn SourceProvider,
    report: &'a mut Report,
    files: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
    events: Vec<Event>,
    defines: HashMap<String, Define>,
}

impl Loader<'_> {
    fn load_file(&mut self, path: PathBuf, included_from: Option<Loc>) {
        let path = normalize(&path);
        if !self.seen.insert(same_file_key(&path)) {
            return; // already included
        }
        let text = match self.provider.read(&path) {
            Ok(text) => text,
            Err(e) => {
                let message = format!("cannot read \"{}\": {e}", path.display());
                match included_from {
                    Some(loc) => self.report.error(loc, message),
                    None => self.report.global_error(message),
                }
                return;
            }
        };
        let file = self.files.len();
        self.files.push(path.clone());

        for (index, raw) in text.lines().enumerate() {
            let line = index as u32 + 1;
            let tokens = match lex_line(raw) {
                Ok(tokens) if tokens.is_empty() => continue,
                Ok(tokens) => tokens,
                Err((col, message)) => {
                    self.report.error(Loc::new(file, line, col), message);
                    continue;
                }
            };
            let loc = Loc::new(file, line, tokens[0].col);
            match &tokens[0].tok {
                Tok::Directive(d) if d == "include" => match tokens.as_slice() {
                    [
                        _,
                        Token {
                            tok: Tok::Str(name),
                            ..
                        },
                    ] => {
                        let name = String::from_utf8_lossy(name).into_owned();
                        let target = path.parent().unwrap_or(Path::new("")).join(name);
                        self.events.push(Event::BlockBreak);
                        self.load_file(target, Some(loc));
                        self.events.push(Event::BlockBreak);
                    }
                    _ => self.report.error(
                        loc,
                        ".include expects a file name in quotes: .include \"file.has\"",
                    ),
                },
                Tok::Directive(d) if d == "define" => self.define(loc, &tokens),
                _ => self.events.push(Event::Line(Line { file, line, tokens })),
            }
        }
    }

    fn define(&mut self, loc: Loc, tokens: &[Token]) {
        let usage = ".define expects a name and a value: .define NAME 0x2A";
        let name = match tokens.get(1) {
            Some(Token {
                tok: Tok::Ident(name),
                ..
            }) if !name.contains('.') => name.clone(),
            _ => return self.report.error(loc, usage),
        };
        if tokens.len() < 3 {
            return self.report.error(loc, usage);
        }
        if is_reserved(&name) {
            return self
                .report
                .error(loc, format!("'{name}' is a reserved word"));
        }
        if let Some(previous) = self.defines.get(&name) {
            let where_ = self.files[previous.loc.file].display();
            let message = format!(
                "'{name}' is already defined ({where_}, line {})",
                previous.loc.line
            );
            return self.report.error(loc, message);
        }
        self.defines.insert(
            name,
            Define {
                tokens: tokens[2..].to_vec(),
                loc,
            },
        );
    }
}

/// Replaces define names by their values, recursively.
fn expand(
    tokens: &[Token],
    defines: &HashMap<String, Define>,
    depth: usize,
) -> Result<Vec<Token>, String> {
    if depth > MAX_DEFINE_DEPTH {
        return Err("defines refer to each other in a loop".into());
    }
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        match &token.tok {
            Tok::Ident(name) if defines.contains_key(name) => {
                let value = expand(&defines[name].tokens, defines, depth + 1)?;
                out.extend(value.into_iter().map(|t| Token {
                    tok: t.tok,
                    col: token.col,
                }));
            }
            _ => out.push(token.clone()),
        }
    }
    Ok(out)
}
