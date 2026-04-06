pub mod error;
pub use error::{
    emit_report, install_diagnostic_hook, print_diagnostic, report_for_span,
    report_for_span_with_help, to_source_span, DiagnosticKind, LexError, LexErrorKind, NimbleError,
    NimbleResult, ParseError, ParseErrorKind, SemanticError, SourceFile,
};
