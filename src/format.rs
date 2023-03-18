use std::{
    fmt::{self, Write},
    fs,
    path::Path,
};

use crate::{
    ast::*,
    compile::{CompileError, Compiler},
    lex::Sp,
    ops::Primitive,
    parse::parse,
    UiuaError, UiuaResult,
};

pub fn format_items(items: Vec<Item>) -> Result<String, Vec<Sp<CompileError>>> {
    let mut state = FormatState {
        string: String::new(),
        was_strand: false,
        compiler: Compiler::new().eval_consts(false),
    };
    for item in items {
        item.format(&mut state);
    }
    if !state.compiler.errors.is_empty() {
        return Err(state.compiler.errors);
    }
    let mut s = state.string;
    s = s.trim_end().into();
    s.push('\n');
    Ok(s)
}

pub fn format(input: &str, path: &Path) -> Result<String, Vec<Sp<CompileError>>> {
    let (items, errors) = parse(input, path);
    let mut errors: Vec<Sp<CompileError>> = errors.into_iter().map(Sp::map_into).collect();
    if errors.is_empty() {
        match format_items(items) {
            Ok(s) => Ok(s),
            Err(e) => {
                errors.extend(e);
                Err(errors)
            }
        }
    } else {
        Err(errors)
    }
}

pub fn format_file<P: AsRef<Path>>(path: P) -> UiuaResult<String> {
    let path = path.as_ref();
    let input = fs::read_to_string(path).map_err(|e| UiuaError::Load(path.to_path_buf(), e))?;
    let formatted = format(&input, path)?;
    if formatted == input {
        return Ok(formatted);
    }
    fs::write(path, &formatted).map_err(|e| UiuaError::Format(path.to_path_buf(), e))?;
    Ok(formatted)
}

struct FormatState {
    pub string: String,
    was_strand: bool,
    compiler: Compiler,
}

impl FormatState {
    fn push<T: fmt::Display>(&mut self, t: T) {
        self.was_strand = false;
        write!(&mut self.string, "{t}").unwrap();
    }
    fn space_if_alphanumeric(&mut self) {
        self.space_if_was_strand();
        if self.string.ends_with(char::is_alphanumeric) {
            self.push(' ');
        }
    }
    fn space_if_alphabetic(&mut self) {
        self.space_if_was_strand();
        if self.string.ends_with(char::is_alphabetic) {
            self.push(' ');
        }
    }
    fn space_if_was_strand(&mut self) {
        if self.was_strand {
            self.push(' ');
        }
    }
}

trait Format {
    fn format(&self, state: &mut FormatState);
}

impl Format for Item {
    fn format(&self, state: &mut FormatState) {
        match self {
            Item::Words(words) => {
                for word in words {
                    word.value.format(state);
                }
            }
            Item::Binding(l) => l.format(state),
            Item::Comment(comment) => {
                state.push("# ");
                state.push(comment);
            }
            Item::Newlines => {}
        }
        state.compiler.item(self.clone());
        state.push('\n');
    }
}

impl Format for Binding {
    fn format(&self, state: &mut FormatState) {
        state.push(&self.name.value);
        state.push(" = ");
        for word in &self.words {
            word.value.format(state);
        }
    }
}

impl Format for Word {
    fn format(&self, state: &mut FormatState) {
        match self {
            Word::Real(f) => {
                state.space_if_alphanumeric();
                state.push(f);
            }
            Word::Char(c) => {
                state.space_if_alphanumeric();
                state.push(&format!("{c:?}"));
            }
            Word::String(s) => {
                state.space_if_alphanumeric();
                state.push(&format!("{s:?}"));
            }
            Word::Ident(ident) => {
                if !state.compiler.is_bound(ident) {
                    if let Some(prim) = Primitive::from_name(ident.as_str()) {
                        return prim.format(state);
                    }
                }
                state.space_if_alphabetic();
                state.push(ident);
            }
            Word::Strand(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        state.push('_');
                    }
                    item.value.format(state);
                }
                state.was_strand = true;
            }
            Word::Array(items) => {
                state.push('[');
                for item in items {
                    item.value.format(state);
                }
                state.push(']');
            }
            Word::Func(f) => {
                state.push('(');
                for word in &f.body {
                    word.value.format(state);
                }
                state.push(')');
            }
            Word::Selector(s) => {
                state.space_if_alphabetic();
                state.push(&s.to_string());
            }
            Word::FuncArray(fs) => {
                state.push('(');
                for (i, f) in fs.iter().enumerate() {
                    if i > 0 {
                        state.push('|');
                    }
                    for word in &f.body {
                        word.value.format(state);
                    }
                }
                state.push(')');
            }
            Word::Primitive(prim) => prim.format(state),
            Word::Modified(m) => {
                m.modifier.value.format(state);
                m.word.value.format(state);
            }
        }
    }
}

impl Format for Primitive {
    fn format(&self, state: &mut FormatState) {
        let s = self.to_string();
        if s.starts_with(char::is_alphabetic) {
            state.space_if_alphanumeric();
        }
        state.push(s);
    }
}
