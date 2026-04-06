use crate::compiler::Compiler;
use crate::error::report_for_span;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::vm::Value;
use crate::vm::VM;
use colored::Colorize;
use miette::Report;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::sync::Arc;

pub fn start() {
    let mut rl = DefaultEditor::new().unwrap();
    let mut vm = VM::new();
    println!("Nimble v0.1.0");
    println!("Type :help for commands");

    loop {
        let readline = rl.readline(">>> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                let _ = rl.add_history_entry(line.as_str());

                if trimmed.starts_with(":globals") {
                    handle_globals(trimmed, &vm);
                    continue;
                }
                if trimmed.starts_with(':') {
                    match trimmed {
                        ":help"     => println!("Commands: :quit, :q, :clear, :globals"),
                        ":quit"|":q"=> break,
                        ":clear"    => { let _ = rl.clear_history(); }
                        _           => println!("Unknown command: {}", trimmed),
                    }
                    continue;
                }
                let _ = execute_line(&line, &mut vm);
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => { eprintln!("REPL Error: {:?}", err); break; }
        }
    }
}

fn execute_line(line: &str, vm: &mut VM) -> Result<(), ()> {
    let source = format!("{}\n", line);
    let mut lexer = Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            let report = report_for_span("<repl>", &source, format!("Lexer error: {}", e.message), e.span, "here");
            eprintln!("{report}");
            return Err(());
        }
    };

    let mut parser = Parser::new(tokens);
    let stmts = match parser.parse() {
        Ok(s) => s,
        Err(errs) => {
            for err in errs {
                let report = report_for_span("<repl>", &source, format!("Parser error: {}", err.message), err.span, "here");
                eprintln!("{report}");
            }
            return Err(());
        }
    };

    let mut compiler = Compiler::new("repl".into());
    let chunk = compiler.compile_stmts(&stmts);

    match vm.run(Arc::clone(&chunk)) {
        Ok(Value::Null) => {}
        Ok(val) => println!("{}", val.stringify().cyan()),
        Err(e) => {
            let report = Report::msg(format!("Runtime error: {}", e));
            eprintln!("{:?}", report);
        }
    }
    Ok(())
}

fn handle_globals(cmd: &str, vm: &VM) {
    let filter = cmd.strip_prefix(":globals").unwrap_or("").trim();
    let entries = vm.global_entries();
    if entries.is_empty() {
        println!("(no globals)");
        return;
    }
    for (name, val) in entries {
        if filter.is_empty() || name.contains(filter) {
            println!("  {} = {}", name.yellow(), val.stringify());
        }
    }
}
