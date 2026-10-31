//! Function signatures of C code, for the parameter hints of the editor:
//! the declarations of the library headers and of the project's files, and
//! the call the cursor is in.
//!
//! Like the highlighter, this is a lenient scanner meant for display only:
//! it never fails, and the compiler stays the judge of what is valid. It
//! does not run the preprocessor (`#ifdef` is not understood).
//!
//! Positions are character indices (not bytes), like the editor's cursor.

use std::collections::HashMap;

/// A function, or a macro with parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    /// Return type (`int`, `char *`), or `#define` for a macro.
    pub returns: String,
    pub name: String,
    /// The parameters as written (`unsigned char x`, `...`); empty for
    /// `(void)` and `()`.
    pub params: Vec<String>,
    /// The comment just above the declaration and the one after it on its
    /// line, without the comment marks.
    pub doc: String,
}

impl Signature {
    pub fn is_macro(&self) -> bool {
        self.returns == "#define"
    }

    /// The parameter the argument `arg` (0 = the first) goes to: the last
    /// one for the extra arguments of `...`, `None` past the end.
    pub fn param_for(&self, arg: usize) -> Option<usize> {
        if arg < self.params.len() {
            Some(arg)
        } else if self.params.last().is_some_and(|p| p == "...") {
            Some(self.params.len() - 1)
        } else {
            None
        }
    }
}

/// The call the cursor is in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub name: String,
    /// The argument the cursor is on (0 = the first).
    pub arg: usize,
    /// Position of the call's `(`.
    pub open: usize,
}

/// Known signatures, by name.
#[derive(Clone, Debug, Default)]
pub struct Signatures(HashMap<String, Signature>);

impl Signatures {
    pub fn get(&self, name: &str) -> Option<&Signature> {
        self.0.get(name)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Adds the declarations of a C file. A name already known is kept,
    /// unless it has no comment and the new one has (a prototype with its
    /// comment in a header, the definition in a `.c` file).
    pub fn add_source(&mut self, source: &str) {
        for signature in scan(source) {
            match self.0.get(&signature.name) {
                Some(known) if !known.doc.is_empty() || signature.doc.is_empty() => {}
                _ => {
                    self.0.insert(signature.name.clone(), signature);
                }
            }
        }
    }

    /// The innermost call around `cursor` whose function is known, with
    /// its signature.
    pub fn call_at(&self, text: &str, cursor: usize) -> Option<(Call, &Signature)> {
        open_calls(text, cursor)
            .into_iter()
            .find_map(|call| self.get(&call.name).map(|s| (call, s)))
    }
}

/// A comment: where it is, and its text without the comment marks.
struct Comment {
    start: usize,
    end: usize,
    text: String,
}

/// The code with comments, strings and character constants blanked out
/// (spaces, newlines kept, so positions stay the same), and the comments.
fn strip(chars: &[char]) -> (Vec<char>, Vec<Comment>) {
    let mut code = chars.to_vec();
    let mut comments = Vec::new();
    let blank = |code: &mut Vec<char>, range: std::ops::Range<usize>| {
        for c in &mut code[range] {
            if *c != '\n' {
                *c = ' ';
            }
        }
    };
    let mut i = 0;
    while i < chars.len() {
        match (chars[i], chars.get(i + 1)) {
            ('/', Some('/')) => {
                let end = chars[i..]
                    .iter()
                    .position(|&c| c == '\n')
                    .map_or(chars.len(), |p| i + p);
                let text: String = chars[i + 2..end].iter().collect();
                comments.push(Comment {
                    start: i,
                    end,
                    text: text.trim().to_owned(),
                });
                blank(&mut code, i..end);
                i = end;
            }
            ('/', Some('*')) => {
                let end = (i + 2..chars.len().saturating_sub(1))
                    .find(|&j| chars[j] == '*' && chars[j + 1] == '/')
                    .map_or(chars.len(), |j| j + 2);
                let inner_end = if end >= i + 4 && chars[end - 1] == '/' {
                    end - 2
                } else {
                    end
                };
                let inner: String = chars[i + 2..inner_end.max(i + 2)].iter().collect();
                comments.push(Comment {
                    start: i,
                    end,
                    text: clean_block(&inner),
                });
                blank(&mut code, i..end);
                i = end;
            }
            (quote @ ('"' | '\''), _) => {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != quote && chars[j] != '\n' {
                    j += if chars[j] == '\\' { 2 } else { 1 };
                }
                let end = (j + 1).min(chars.len());
                blank(&mut code, i..end);
                i = end;
            }
            _ => i += 1,
        }
    }
    (code, comments)
}

/// The text of a `/* */` comment: the lines without their leading `*`,
/// rows of dashes taken away.
fn clean_block(inner: &str) -> String {
    let lines: Vec<String> = inner
        .lines()
        .map(|line| {
            let line = line.trim();
            let line = line.strip_prefix('*').unwrap_or(line);
            let mut line = line.trim();
            if line.starts_with("---") {
                line = line.trim_start_matches('-');
            }
            if line.ends_with("---") {
                line = line.trim_end_matches('-');
            }
            line.trim().to_owned()
        })
        .collect();
    lines.join("\n").trim().to_owned()
}

/// The lines of comments joined into paragraphs: a blank line starts a
/// new paragraph, other line breaks become spaces.
fn reflow(text: &str) -> String {
    let mut out = String::new();
    let mut blank_before = false;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            blank_before = !out.is_empty();
            continue;
        }
        if blank_before {
            out.push_str("\n\n");
        } else if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(line);
        blank_before = false;
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The name that ends just before `end` (spaces skipped), and where it starts.
fn name_before(code: &[char], end: usize) -> Option<(String, usize)> {
    let end = code[..end].iter().rposition(|&c| !c.is_whitespace())? + 1;
    let start = code[..end]
        .iter()
        .rposition(|&c| !is_ident(c))
        .map_or(0, |p| p + 1);
    let name: String = code[start..end].iter().collect();
    let first = name.chars().next()?;
    (!first.is_ascii_digit()).then_some((name, start))
}

/// Words that come before `(` without being a function.
const KEYWORDS: [&str; 9] = [
    "if", "while", "for", "switch", "return", "sizeof", "do", "else", "case",
];

/// The `)` matching the `(` at `open`, if any.
fn matching_paren(code: &[char], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, &c) in code.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Parameters between the brackets: split at the commas outside inner
/// brackets, spaces tidied.
fn split_params(code: &[char]) -> Vec<String> {
    let mut params = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for &c in code {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                params.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    params.push(current);
    let params: Vec<String> = params.iter().map(|p| tidy(p)).collect();
    if params.len() == 1 && (params[0].is_empty() || params[0] == "void") {
        Vec::new()
    } else {
        params
    }
}

/// Spaces and line breaks reduced to single spaces.
fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The comments of a declaration: the one(s) right above `start` (nothing
/// but spaces and at most one line break between them), and the one after
/// `end` on the same line.
fn doc_for(
    chars: &[char],
    comments: &[Comment],
    start: usize,
    end: Option<usize>,
) -> (String, String) {
    let gap_ok = |from: usize, to: usize, max_newlines: usize| {
        from <= to
            && chars[from..to].iter().all(|c| c.is_whitespace())
            && chars[from..to].iter().filter(|&&c| c == '\n').count() <= max_newlines
    };
    // A comment after some code on its line is about that code.
    let starts_line = |comment: &Comment| {
        chars[..comment.start]
            .iter()
            .rev()
            .take_while(|&&c| c != '\n')
            .all(|c| c.is_whitespace())
    };
    let mut above = Vec::new();
    let mut edge = start;
    for comment in comments.iter().rev().filter(|c| c.end <= start) {
        if !gap_ok(comment.end, edge, 1) || !starts_line(comment) {
            break;
        }
        above.push(comment.text.as_str());
        edge = comment.start;
    }
    above.reverse();
    let above = reflow(&above.join("\n"));
    let after = match end {
        Some(end) => comments
            .iter()
            .find(|c| c.start >= end)
            .filter(|c| gap_ok(end, c.start, 0))
            .map(|c| reflow(&c.text))
            .unwrap_or_default(),
        None => String::new(),
    };
    (above, after)
}

/// A short comment with a capital and no full stop (`Text mode`) is the
/// title of the declarations below it, not a description.
fn is_title(comment: &str) -> bool {
    comment.split_whitespace().count() <= 3
        && comment.starts_with(|c: char| c.is_uppercase())
        && !comment.ends_with(['.', ')'])
}

/// A declaration found by the scanner, before its comments are chosen.
struct Found {
    signature: Signature,
    /// Where the declaration starts.
    start: usize,
    /// Where a prototype or a macro ends (after its `;` or its line); `None`
    /// for a definition (with a body).
    end: Option<usize>,
}

/// The function declarations and definitions at the top level of a C file,
/// and its macros with parameters.
pub fn scan(source: &str) -> Vec<Signature> {
    let chars: Vec<char> = source.chars().collect();
    let (code, comments) = strip(&chars);
    let mut found: Vec<Found> = Vec::new();
    let mut braces = 0usize;
    let mut parens = 0usize;
    // Where the current top-level statement starts.
    let mut statement = 0;
    let mut line_start = true;
    let mut i = 0;
    while i < code.len() {
        let c = code[i];
        if line_start && c == '#' {
            // A preprocessor line, with its "\" continuations.
            let mut end = i;
            while end < code.len() && (code[end] != '\n' || (end > 0 && code[end - 1] == '\\')) {
                end += 1;
            }
            if let Some(signature) = define(&code, i, end) {
                found.push(Found {
                    signature,
                    start: i,
                    end: Some(end),
                });
            }
            i = end;
            statement = end;
            continue;
        }
        if c == '\n' {
            line_start = true;
        } else if !c.is_whitespace() {
            line_start = false;
        }
        match c {
            '{' => {
                braces += 1;
                if braces == 1 {
                    statement = i + 1;
                }
            }
            '}' => {
                braces = braces.saturating_sub(1);
                if braces == 0 {
                    statement = i + 1;
                }
            }
            ';' if braces == 0 => statement = i + 1,
            '(' if braces == 0 && parens == 0 => {
                if let Some((declaration, close)) = function(&code, statement, i) {
                    found.push(declaration);
                    i = close + 1;
                    continue;
                }
                parens += 1;
            }
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }

    // The comments. Declarations on the lines right below one with a
    // comment above it, and without their own, share that comment
    // ("Smaller, larger: min / max..." above min, max, umin and umax).
    let mut group: Option<(usize, String)> = None;
    let mut signatures = Vec::new();
    for Found {
        mut signature,
        start,
        end,
    } in found
    {
        let (mut above, after) = doc_for(&chars, &comments, start, end);
        if is_title(&above) {
            above.clear();
        }
        if above.is_empty()
            && let Some((group_end, group_doc)) = &group
            && *group_end <= start
            && code[*group_end..start].iter().all(|c| c.is_whitespace())
            && code[*group_end..start]
                .iter()
                .filter(|&&c| c == '\n')
                .count()
                == 1
        {
            above = group_doc.clone();
        }
        group = end.map(|end| (end, above.clone()));
        signature.doc = match (above.is_empty(), after.is_empty()) {
            (_, true) => above,
            (true, false) => after,
            (false, false) => format!("{above}\n\n{after}"),
        };
        signatures.push(signature);
    }
    signatures
}

/// The function whose parameters open at `open`, in the statement that
/// starts at `statement`; and the position of its `)`.
fn function(code: &[char], statement: usize, open: usize) -> Option<(Found, usize)> {
    let (name, name_start) = name_before(code, open)?;
    let prefix: String = code[statement..name_start].iter().collect();
    let returns = tidy(&prefix);
    let returns = returns
        .strip_prefix("extern ")
        .unwrap_or(&returns)
        .to_owned();
    let last = returns.chars().last()?;
    if KEYWORDS.contains(&name.as_str())
        || !(is_ident(last) || last == '*')
        || returns.contains(['=', '(', ')', ',', '#'])
        || returns.starts_with("typedef")
    {
        return None;
    }
    let close = matching_paren(code, open)?;
    let after = code[close + 1..].iter().position(|c| !c.is_whitespace())?;
    let after_pos = close + 1 + after;
    let end = match code[after_pos] {
        ';' => Some(after_pos + 1),
        '{' => None,
        _ => return None,
    };
    let start = statement + code[statement..].iter().position(|c| !c.is_whitespace())?;
    Some((
        Found {
            signature: Signature {
                returns,
                name,
                params: split_params(&code[open + 1..close]),
                doc: String::new(),
            },
            start,
            end,
        },
        close,
    ))
}

/// `#define NAME(params)` on the line from `hash` to `end`.
fn define(code: &[char], hash: usize, end: usize) -> Option<Signature> {
    let line: String = code[hash + 1..end].iter().collect();
    let rest = line.trim_start().strip_prefix("define")?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    let name_len = rest.find(|c: char| !is_ident(c))?;
    let (name, after) = rest.split_at(name_len);
    // A macro with parameters has its "(" right after the name.
    let params = after.strip_prefix('(')?;
    let params: Vec<char> = params[..params.find(')')?].chars().collect();
    Some(Signature {
        returns: "#define".to_owned(),
        name: name.to_owned(),
        params: split_params(&params),
        doc: String::new(),
    })
}

/// The calls whose `(` is still open at `cursor`, innermost first. A `;`
/// or a brace ends them all.
pub fn open_calls(text: &str, cursor: usize) -> Vec<Call> {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let (code, _) = strip(&chars);
    // (position of the "(", commas seen at its level)
    let mut open: Vec<(usize, usize)> = Vec::new();
    for (i, &c) in code[..cursor].iter().enumerate() {
        match c {
            '(' => open.push((i, 0)),
            ')' => {
                open.pop();
            }
            ',' => {
                if let Some(last) = open.last_mut() {
                    last.1 += 1;
                }
            }
            ';' | '{' | '}' => open.clear(),
            _ => {}
        }
    }
    open.iter()
        .rev()
        .filter_map(|&(paren, commas)| {
            let (name, _) = name_before(&code, paren)?;
            Some(Call {
                name,
                arg: commas,
                open: paren,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(source: &str) -> Signature {
        let found = scan(source);
        assert_eq!(found.len(), 1, "{found:?}");
        found.into_iter().next().unwrap()
    }

    #[test]
    fn prototypes_with_their_comments() {
        let s = one("/* Reads a line\n   typed. */\nchar *term_readline(void);\n");
        assert_eq!(s.returns, "char *");
        assert_eq!(s.name, "term_readline");
        assert!(s.params.is_empty());
        assert_eq!(s.doc, "Reads a line typed.");

        let s =
            one("int screen_putc(char c, unsigned char x,\n    unsigned char y);  /* 0, or 1 */");
        assert_eq!(s.params, ["char c", "unsigned char x", "unsigned char y"]);
        assert_eq!(s.doc, "0, or 1");

        // Both: the comment above, then the one after.
        let s = one("// Above it.\nint f(int a); // after\n");
        assert_eq!(s.doc, "Above it.\n\nafter");

        // A blank line between: a section title, not about this function.
        let s = one("/* --- Terminal --- */\n\nvoid term_clear(void);");
        assert_eq!(s.doc, "");
        // The comment of the next line is not taken.
        let s = one("void a(void);\n/* about b */\n");
        assert_eq!(s.doc, "");
    }

    #[test]
    fn definitions_and_what_is_not_a_function() {
        let s = one("static int add(int a, int b)\n{\n    return f(a, b);\n}\n");
        assert_eq!((s.returns.as_str(), s.name.as_str()), ("static int", "add"));
        assert_eq!(s.params, ["int a", "int b"]);
        // Calls inside the body are not declarations.
        assert_eq!(scan("void g(void) { int x = h(1); }").len(), 1);
        assert!(scan("int x = f(3);").is_empty());
        assert!(scan("typedef void (*handler)(void);").is_empty());
        assert!(scan("struct s { int a; };").is_empty());
        // Parentheses in strings and comments do not count.
        assert_eq!(
            scan("char *s = \"f(\";\nint g(int a /* ( */);")[0].name,
            "g"
        );
    }

    #[test]
    fn variadic_and_function_pointer_params() {
        let s = one("int printf(const char *fmt, ...);");
        assert_eq!(s.params, ["const char *fmt", "..."]);
        assert_eq!(s.param_for(0), Some(0));
        assert_eq!(s.param_for(5), Some(1));
        let s = one("int set_vector(unsigned char port, void (*handler)(void));");
        assert_eq!(s.params, ["unsigned char port", "void (*handler)(void)"]);
        assert_eq!(s.param_for(2), None);
    }

    #[test]
    fn macros_with_parameters() {
        let found = scan(
            "/* Colors of a cell. */\n#define COLORS(character, background) (((background) << 4) | (character))\n#define BLACK 0\n#define F (x)\n",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "COLORS");
        assert!(found[0].is_macro());
        assert_eq!(found[0].params, ["character", "background"]);
        assert_eq!(found[0].doc, "Colors of a cell.");
    }

    #[test]
    fn call_at_the_cursor() {
        let mut known = Signatures::default();
        known.add_source("int f(int a, int b);\nint g(int x);");
        let text = "int main(void)\n{\n    f(1, g(2), (3 + 4)";
        let at = |s: &str| text.find(s).unwrap() + s.len();
        let call = |cursor| {
            known
                .call_at(text, cursor)
                .map(|(c, s)| (c.name, c.arg, s.name.clone()))
        };
        assert_eq!(call(at("f(")), Some(("f".into(), 0, "f".into())));
        assert_eq!(call(at("f(1, ")), Some(("f".into(), 1, "f".into())));
        // Inside g's brackets: g; after them, f again.
        assert_eq!(call(at("g(")), Some(("g".into(), 0, "g".into())));
        assert_eq!(call(at("g(2), ")), Some(("f".into(), 2, "f".into())));
        // Plain brackets inside a call: still that call.
        assert_eq!(call(at("(3 ")), Some(("f".into(), 2, "f".into())));
        // Before any call, and the declaration of main (unknown).
        assert_eq!(call(at("main(")), None);
        assert_eq!(call(0), None);
        // A closed call, or one cut by a ";".
        assert_eq!(known.call_at("f(1);", 5), None);
        assert_eq!(known.call_at("f(1; ", 5), None);
        // Commas and brackets in strings do not count.
        assert_eq!(known.call_at("f(\",(\", ", 8).map(|(c, _)| c.arg), Some(1));
    }

    #[test]
    fn shared_comments_and_titles() {
        let found = scan(
            "/* Smaller, larger. */\nint min(int a, int b);\nint max(int a, int b);  /* ints */\n\nint other(void);\n/* Text mode */\nint putc(char c);\n/* -1 when off --- */\nint getc(void);",
        );
        let doc = |name: &str| found.iter().find(|s| s.name == name).unwrap().doc.clone();
        assert_eq!(doc("min"), "Smaller, larger.");
        assert_eq!(doc("max"), "Smaller, larger.\n\nints");
        // After a blank line: not in the group.
        assert_eq!(doc("other"), "");
        // A title is not a description.
        assert_eq!(doc("putc"), "");
        // Dashes: only rows of them go.
        assert_eq!(doc("getc"), "-1 when off");
    }

    #[test]
    fn the_documented_prototype_wins() {
        let mut known = Signatures::default();
        known.add_source("int f(int a)\n{\n}\n");
        known.add_source("/* Doubles a. */\nint f(int a);");
        known.add_source("int f(int z);");
        assert_eq!(known.get("f").unwrap().doc, "Doubles a.");
        assert_eq!(known.len(), 1);
    }

    #[test]
    fn library_headers() {
        // Every header of the C library: its functions are found.
        let include = hbc2_cc::lib_dir().join("include");
        let mut known = Signatures::default();
        for entry in std::fs::read_dir(&include).unwrap() {
            let path = entry.unwrap().path();
            known.add_source(&std::fs::read_to_string(path).unwrap());
        }
        for name in [
            "printf",
            "term_readline",
            "screen_putc",
            "strcmp",
            "rand_range",
            "COLORS",
        ] {
            assert!(known.get(name).is_some(), "{name} not found");
        }
        let putc = known.get("screen_putc").unwrap();
        assert_eq!(putc.params.len(), 4);
        assert!(
            known
                .get("term_readline")
                .unwrap()
                .doc
                .starts_with("Reads a line")
        );
    }
}
