use crate::compiler::bytecode::FunctionChunk;
use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ModuleResolver {
    stdlib_path: PathBuf,
}

impl ModuleResolver {
    pub fn new() -> Self {
        let mut candidates = Vec::new();

        if let Ok(current_dir) = std::env::current_dir() {
            candidates.push(current_dir.join("stdlib"));
        }

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidates.push(exe_dir.join("stdlib"));
                if let Some(target_dir) = exe_dir.parent() {
                    candidates.push(target_dir.join("stdlib"));
                    if let Some(repo_root) = target_dir.parent() {
                        candidates.push(repo_root.join("stdlib"));
                    }
                }
            }
        }

        candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("stdlib"));

        let stdlib_path = candidates
            .into_iter()
            .find(|path| path.exists())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("stdlib"));

        Self { stdlib_path }
    }

    pub fn resolve(
        &self,
        source: &str,
        current_dir: &Path,
    ) -> Result<(Arc<FunctionChunk>, PathBuf), String> {
        let path = if source.starts_with('.') {
            let mut p = current_dir.to_path_buf();
            p.push(source);
            if p.is_dir() {
                p.push("mod.nmb");
            } else {
                p.set_extension("nmb");
            }
            p
        } else {
            let mut p = self.stdlib_path.clone();
            p.push(source);
            p.push("mod.nmb");
            p
        };

        if !path.exists() {
            // Try alternate: stdlib/<name>.nmb
            let mut alt = self.stdlib_path.clone();
            alt.push(source);
            alt.set_extension("nmb");
            if alt.exists() {
                let content = fs::read_to_string(&alt).map_err(|e| e.to_string())?;
                let chunk = self.compile_module(&content, source)?;
                return Ok((chunk, alt));
            }
            return Err(format!(
                "Module '{}' not found (looked in {})",
                source,
                path.display()
            ));
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let chunk = self.compile_module(&content, source)?;
        Ok((chunk, path))
    }

    fn compile_module(&self, content: &str, name: &str) -> Result<Arc<FunctionChunk>, String> {
        let mut lexer = Lexer::new(content);
        let tokens = lexer
            .tokenize()
            .map_err(|e| format!("Lexer error in module '{}': {}", name, e.message))?;
        let mut parser = Parser::new(tokens);
        let stmts = match parser.parse() {
            Ok(s) => s,
            Err(errs) => {
                let first = errs.into_iter().next().unwrap();
                return Err(format!(
                    "Parser error in module '{}': {}",
                    name, first.message
                ));
            }
        };
        let mut compiler = Compiler::new(name.to_string());
        Ok(compiler.compile_stmts(&stmts))
    }
}
