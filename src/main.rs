use clap::{Parser as ClapParser, Subcommand};
use colored::Colorize;
use nimble::compiler::Compiler;
use nimble::error::{emit_report, install_diagnostic_hook, NimbleError, NimbleResult, SourceFile};
use nimble::lexer::Lexer;
use nimble::parser::{ast::Stmt, Parser};
use nimble::repl;
use nimble::types::infer::Inferencer;
use nimble::vm::VM;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(ClapParser)]
#[command(name = "nimble")]
#[command(about = "The Nimble Programming Language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        file: PathBuf,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Check {
        file: PathBuf,
    },
    Repl,
    Version,
}

fn main() {
    install_diagnostic_hook();
    if let Err(report) = try_main() {
        emit_report(&report);
        std::process::exit(1);
    }
}

fn try_main() -> NimbleResult<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Run { file, args }) => run_file(&file, args),
        Some(Commands::Check { file }) => check_file(&file),
        Some(Commands::Repl) | None => repl::repl::start(),
        Some(Commands::Version) => {
            println!("Nimble v0.1.0");
            Ok(())
        }
    }
}

fn read_source_file(path: &PathBuf) -> NimbleResult<SourceFile> {
    let source = fs::read_to_string(path).map_err(|source| {
        miette::Report::new(NimbleError::Io {
            path: path.display().to_string(),
            source,
        })
    })?;
    Ok(SourceFile::new(path.display().to_string(), source))
}

fn parse_source(source_file: &SourceFile) -> NimbleResult<Vec<Stmt>> {
    let mut lexer = Lexer::new(source_file.source());
    let tokens = lexer
        .tokenize()
        .map_err(|error| miette::Report::new(NimbleError::from_lex(source_file, error)))?;

    let mut parser = Parser::new(tokens);
    parser.parse().map_err(|errors| {
        let mut diagnostics = errors
            .into_iter()
            .map(|error| NimbleError::from_parse(source_file, error))
            .collect::<Vec<_>>();

        if diagnostics.len() == 1 {
            miette::Report::new(diagnostics.remove(0))
        } else {
            miette::Report::new(NimbleError::multiple(
                source_file,
                format!("failed to parse `{}`", source_file.name()),
                diagnostics,
            ))
        }
    })
}

fn type_check(source_file: &SourceFile, stmts: &[Stmt]) -> NimbleResult<()> {
    let mut inferencer = Inferencer::new();
    inferencer
        .infer_stmts(stmts)
        .map_err(|error| miette::Report::new(NimbleError::from_semantic(source_file, error)))
}

fn check_file(path: &PathBuf) -> NimbleResult<()> {
    println!(
        "{}",
        format!("Checking {} ...", path.display())
            .bright_blue()
            .bold()
    );
    let source_file = read_source_file(path)?;
    let stmts = parse_source(&source_file)?;
    type_check(&source_file, &stmts)?;
    println!("{}", "Type check passed.".green().bold());
    Ok(())
}

fn run_file(path: &PathBuf, script_args: Vec<String>) -> NimbleResult<()> {
    let source_file = read_source_file(path)?;
    run_source(
        &source_file,
        Some(path.parent().unwrap_or_else(|| Path::new("."))),
        script_args,
    )
}

fn run_source(
    source_file: &SourceFile,
    working_dir: Option<&Path>,
    script_args: Vec<String>,
) -> NimbleResult<()> {
    let stmts = parse_source(source_file)?;
    type_check(source_file, &stmts)?;

    let mut compiler = Compiler::new("main".into());
    let chunk = compiler.compile_stmts(&stmts);

    let mut vm = VM::new();
    vm.set_script_args(script_args);
    let result = if let Some(dir) = working_dir {
        vm.run_with_dir(Arc::clone(&chunk), dir.to_path_buf())
    } else {
        vm.run(Arc::clone(&chunk))
    };

    result
        .map(|_| ())
        .map_err(|message| miette::Report::new(NimbleError::runtime(source_file, message)))
}
