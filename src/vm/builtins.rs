use crate::vm::{Value, VM};
use libffi::middle::{Arg, Cif, CodePtr, Type as LibffiType};
use libloading::Library;
use rand::Rng;
use std::cmp::Ordering;
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::mem;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn out(_vm: &mut VM, args: Vec<Value>) -> Value {
    let mut first = true;
    for arg in args {
        if !first {
            print!(" ");
        }
        first = false;
        print!("{}", arg.stringify());
    }
    println!();
    Value::Null
}

pub fn input(_vm: &mut VM, args: Vec<Value>) -> Value {
    if let Some(Value::Str(s)) = args.get(0) {
        print!("{}", s);
        io::stdout().flush().unwrap();
    }
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    Value::Str(Arc::new(input.trim().to_string()))
}

pub fn len(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.is_empty() {
        return Value::Int(0);
    }
    let res = match &args[0] {
        Value::Str(s) => s.chars().count() as i64,
        Value::List(l) => l.lock().unwrap().len() as i64,
        Value::Map(m) => m.lock().unwrap().len() as i64,
        Value::Range { start, end } => (end - start).max(0),
        _ => 0,
    };
    Value::Int(res)
}

pub fn to_int(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.is_empty() {
        return Value::Int(0);
    }
    match &args[0] {
        Value::Int(n) => Value::Int(*n),
        Value::Float(f) => Value::Int(*f as i64),
        Value::Bool(b) => Value::Int(if *b { 1 } else { 0 }),
        Value::Str(s) => Value::Int(s.parse().unwrap_or(0)),
        _ => Value::Int(0),
    }
}

pub fn index_of(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Int(-1);
    }
    let s = match &args[0] {
        Value::Str(s) => s.as_str(),
        _ => return Value::Int(-1),
    };
    let sub = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Value::Int(-1),
    };
    if let Some(idx) = s.find(sub) {
        Value::Int(idx as i64)
    } else {
        Value::Int(-1)
    }
}

pub fn builtin_str_index_of(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Int(-1);
    }
    let s = match &args[0] {
        Value::Str(s) => s.as_str(),
        _ => return Value::Int(-1),
    };
    let sub = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Value::Int(-1),
    };
    if let Some(idx) = s.find(sub) {
        Value::Int(idx as i64)
    } else {
        Value::Int(-1)
    }
}

pub fn error(_vm: &mut VM, args: Vec<Value>) -> Value {
    let msg = if let Some(v) = args.get(0) {
        v.stringify()
    } else {
        "error".into()
    };
    Value::Error(Arc::new(msg))
}

pub fn builtin_file_exists(_vm: &mut VM, args: Vec<Value>) -> Value {
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Bool(false),
    };
    Value::Bool(fs::metadata(path).is_ok())
}

pub fn builtin_io_read_file(_vm: &mut VM, args: Vec<Value>) -> Value {
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    match fs::read_to_string(path) {
        Ok(s) => Value::Str(Arc::new(s)),
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_write_file(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("write_file expects path and content".into()));
    }
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    let content = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("content must be string".into())),
    };
    match fs::write(path, content) {
        Ok(_) => Value::Null,
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_append_file(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("append_file expects path and content".into()));
    }
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    let content = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("content must be string".into())),
    };
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => match f.write_all(content.as_bytes()) {
            Ok(_) => Value::Null,
            Err(e) => Value::Error(Arc::new(e.to_string())),
        },
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_delete_file(_vm: &mut VM, args: Vec<Value>) -> Value {
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    match fs::remove_file(path) {
        Ok(_) => Value::Null,
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_read_lines(_vm: &mut VM, args: Vec<Value>) -> Value {
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    match fs::read_to_string(path) {
        Ok(s) => {
            let mut out = Vec::new();
            for line in s.lines() {
                out.push(Value::Str(Arc::new(line.to_string())));
            }
            Value::List(Arc::new(Mutex::new(out)))
        }
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_write_lines(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("write_lines expects path and lines".into()));
    }
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    let list = match args.get(1) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("lines must be list".into())),
    };
    let mut out = String::new();
    for (i, v) in list.into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&v.stringify());
    }
    match fs::write(path, out) {
        Ok(_) => Value::Null,
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_read_bytes(_vm: &mut VM, args: Vec<Value>) -> Value {
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    match fs::read(path) {
        Ok(bytes) => {
            let mut out = Vec::with_capacity(bytes.len());
            for b in bytes {
                out.push(Value::Int(b as i64));
            }
            Value::List(Arc::new(Mutex::new(out)))
        }
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_write_bytes(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("write_bytes expects path and data".into()));
    }
    let path = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("path must be string".into())),
    };
    let list = match args.get(1) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("data must be list".into())),
    };
    let mut bytes = Vec::with_capacity(list.len());
    for v in list {
        match v {
            Value::Int(n) if (0..=255).contains(&n) => bytes.push(n as u8),
            _ => return Value::Error(Arc::new("data must be list of ints 0..255".into())),
        }
    }
    match fs::write(path, bytes) {
        Ok(_) => Value::Null,
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_copy_file(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("copy_file expects src and dst".into()));
    }
    let src = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("src must be string".into())),
    };
    let dst = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("dst must be string".into())),
    };
    match fs::copy(src, dst) {
        Ok(_) => Value::Null,
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_io_stderr(_vm: &mut VM, args: Vec<Value>) -> Value {
    let mut first = true;
    for arg in args {
        if !first {
            eprint!(" ");
        }
        first = false;
        eprint!("{}", arg.stringify());
    }
    eprintln!();
    Value::Null
}

pub fn builtin_time_now(_vm: &mut VM, _args: Vec<Value>) -> Value {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Value::Int(since_the_epoch.as_millis() as i64)
}

pub fn builtin_time_sleep(_vm: &mut VM, args: Vec<Value>) -> Value {
    let ms = match args.get(0) {
        Some(Value::Int(n)) => *n,
        _ => 0,
    };
    if ms > 0 {
        thread::sleep(Duration::from_millis(ms as u64));
    }
    Value::Null
}

#[derive(Clone, Copy, Debug)]
enum FfiTypeDesc {
    Void,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Isize,
    Usize,
    F32,
    F64,
    Pointer,
    CString,
}

struct CStringArg {
    _buf: CString,
    ptr:  *const c_char,
}

enum FfiArgStorage {
    Bool(u8),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Pointer(*const c_void),
    CString(CStringArg),
}

impl FfiArgStorage {
    fn as_arg(&self) -> Arg {
        match self {
            Self::Bool(v) => Arg::new(v),
            Self::I8(v) => Arg::new(v),
            Self::U8(v) => Arg::new(v),
            Self::I16(v) => Arg::new(v),
            Self::U16(v) => Arg::new(v),
            Self::I32(v) => Arg::new(v),
            Self::U32(v) => Arg::new(v),
            Self::I64(v) => Arg::new(v),
            Self::U64(v) => Arg::new(v),
            Self::F32(v) => Arg::new(v),
            Self::F64(v) => Arg::new(v),
            Self::Pointer(v) => Arg::new(v),
            Self::CString(v) => Arg::new(&v.ptr),
        }
    }
}

impl FfiTypeDesc {
    fn from_name(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "void" | "null" => Ok(Self::Void),
            "bool" => Ok(Self::Bool),
            "i8" => Ok(Self::I8),
            "u8" => Ok(Self::U8),
            "i16" => Ok(Self::I16),
            "u16" => Ok(Self::U16),
            "i32" => Ok(Self::I32),
            "u32" => Ok(Self::U32),
            "i64" | "int64" => Ok(Self::I64),
            "u64" | "uint64" => Ok(Self::U64),
            "isize" => Ok(Self::Isize),
            "usize" => Ok(Self::Usize),
            "f32" | "float" => Ok(Self::F32),
            "f64" | "double" => Ok(Self::F64),
            "ptr" | "pointer" => Ok(Self::Pointer),
            "str" | "cstring" => Ok(Self::CString),
            other => Err(format!("unsupported FFI type '{other}'")),
        }
    }

    fn ffi_type(&self) -> LibffiType {
        match self {
            Self::Void => LibffiType::void(),
            Self::Bool => LibffiType::u8(),
            Self::I8 => LibffiType::i8(),
            Self::U8 => LibffiType::u8(),
            Self::I16 => LibffiType::i16(),
            Self::U16 => LibffiType::u16(),
            Self::I32 => LibffiType::i32(),
            Self::U32 => LibffiType::u32(),
            Self::I64 | Self::Isize => {
                if mem::size_of::<isize>() == 8 {
                    LibffiType::i64()
                } else {
                    LibffiType::i32()
                }
            }
            Self::U64 | Self::Usize => {
                if mem::size_of::<usize>() == 8 {
                    LibffiType::u64()
                } else {
                    LibffiType::u32()
                }
            }
            Self::F32 => LibffiType::f32(),
            Self::F64 => LibffiType::f64(),
            Self::Pointer | Self::CString => LibffiType::pointer(),
        }
    }
}

fn ffi_string(value: &Value, ctx: &str) -> Result<String, String> {
    match value {
        Value::Str(s) => Ok(s.to_string()),
        _ => Err(format!("{ctx} must be string")),
    }
}

fn ffi_list(value: &Value, ctx: &str) -> Result<Vec<Value>, String> {
    match value {
        Value::List(items) => Ok(items.lock().unwrap().clone()),
        _ => Err(format!("{ctx} must be list")),
    }
}

fn ffi_library_path(value: &Value) -> Result<String, String> {
    match value {
        Value::FfiLibrary(path) | Value::Str(path) => Ok(path.to_string()),
        _ => Err("library must be an ffi handle or string path".into()),
    }
}

fn resolve_library_path(vm: &VM, path: &str) -> String {
    let library_path = PathBuf::from(path);
    if library_path.is_absolute() {
        return library_path.to_string_lossy().into_owned();
    }

    let has_explicit_relative =
        path.starts_with('.') || path.contains('/') || path.contains('\\');
    let module_relative = vm.current_module_dir_path().join(&library_path);

    if has_explicit_relative || module_relative.exists() {
        module_relative.to_string_lossy().into_owned()
    } else {
        path.to_string()
    }
}

fn parse_ffi_type_list(value: &Value, ctx: &str) -> Result<Vec<FfiTypeDesc>, String> {
    ffi_list(value, ctx)?
        .into_iter()
        .map(|item| match item {
            Value::Str(name) => FfiTypeDesc::from_name(&name),
            _ => Err(format!("{ctx} entries must be strings")),
        })
        .collect()
}

fn default_c_library_name() -> Result<String, String> {
    if cfg!(target_os = "windows") {
        Ok("msvcrt.dll".into())
    } else if cfg!(target_os = "macos") {
        Ok("/usr/lib/libSystem.B.dylib".into())
    } else if cfg!(target_os = "linux") {
        Ok("libc.so.6".into())
    } else {
        Err("ffi.default_c is not supported on this platform".into())
    }
}

fn platform_library_name(base: &str) -> Result<String, String> {
    let lower = base.to_ascii_lowercase();
    if lower.ends_with(".dll") || lower.ends_with(".so") || lower.ends_with(".dylib") {
        return Ok(base.to_string());
    }

    if cfg!(target_os = "windows") {
        Ok(format!("{base}.dll"))
    } else if cfg!(target_os = "macos") {
        let prefix = if base.starts_with("lib") { "" } else { "lib" };
        Ok(format!("{prefix}{base}.dylib"))
    } else if cfg!(target_os = "linux") {
        let prefix = if base.starts_with("lib") { "" } else { "lib" };
        Ok(format!("{prefix}{base}.so"))
    } else {
        Err("ffi.library_name is not supported on this platform".into())
    }
}

fn validate_ffi_library(path: &str) -> Result<(), String> {
    unsafe { Library::new(path) }
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn build_ffi_arg(expected: FfiTypeDesc, value: &Value) -> Result<FfiArgStorage, String> {
    match expected {
        FfiTypeDesc::Void => Err("void is not a valid argument type".into()),
        FfiTypeDesc::Bool => match value {
            Value::Bool(v) => Ok(FfiArgStorage::Bool(if *v { 1 } else { 0 })),
            Value::Int(v) => Ok(FfiArgStorage::Bool(if *v == 0 { 0 } else { 1 })),
            _ => Err("bool argument must be bool or int".into()),
        },
        FfiTypeDesc::I8 => match value {
            Value::Int(v) => i8::try_from(*v)
                .map(FfiArgStorage::I8)
                .map_err(|_| "i8 argument out of range".into()),
            _ => Err("i8 argument must be int".into()),
        },
        FfiTypeDesc::U8 => match value {
            Value::Int(v) => u8::try_from(*v)
                .map(FfiArgStorage::U8)
                .map_err(|_| "u8 argument out of range".into()),
            _ => Err("u8 argument must be int".into()),
        },
        FfiTypeDesc::I16 => match value {
            Value::Int(v) => i16::try_from(*v)
                .map(FfiArgStorage::I16)
                .map_err(|_| "i16 argument out of range".into()),
            _ => Err("i16 argument must be int".into()),
        },
        FfiTypeDesc::U16 => match value {
            Value::Int(v) => u16::try_from(*v)
                .map(FfiArgStorage::U16)
                .map_err(|_| "u16 argument out of range".into()),
            _ => Err("u16 argument must be int".into()),
        },
        FfiTypeDesc::I32 => match value {
            Value::Int(v) => i32::try_from(*v)
                .map(FfiArgStorage::I32)
                .map_err(|_| "i32 argument out of range".into()),
            _ => Err("i32 argument must be int".into()),
        },
        FfiTypeDesc::U32 => match value {
            Value::Int(v) => u32::try_from(*v)
                .map(FfiArgStorage::U32)
                .map_err(|_| "u32 argument out of range".into()),
            _ => Err("u32 argument must be int".into()),
        },
        FfiTypeDesc::I64 | FfiTypeDesc::Isize => match value {
            Value::Int(v) => Ok(FfiArgStorage::I64(*v)),
            _ => Err("i64/isize argument must be int".into()),
        },
        FfiTypeDesc::U64 | FfiTypeDesc::Usize => match value {
            Value::Int(v) => u64::try_from(*v)
                .map(FfiArgStorage::U64)
                .map_err(|_| "u64/usize argument out of range".into()),
            _ => Err("u64/usize argument must be int".into()),
        },
        FfiTypeDesc::F32 => match value {
            Value::Float(v) => Ok(FfiArgStorage::F32(*v as f32)),
            Value::Int(v) => Ok(FfiArgStorage::F32(*v as f32)),
            _ => Err("f32 argument must be number".into()),
        },
        FfiTypeDesc::F64 => match value {
            Value::Float(v) => Ok(FfiArgStorage::F64(*v)),
            Value::Int(v) => Ok(FfiArgStorage::F64(*v as f64)),
            _ => Err("f64 argument must be number".into()),
        },
        FfiTypeDesc::Pointer => match value {
            Value::Null => Ok(FfiArgStorage::Pointer(std::ptr::null())),
            Value::Int(v) if *v >= 0 => Ok(FfiArgStorage::Pointer(*v as usize as *const c_void)),
            Value::Int(_) => Err("pointer argument must be null or a non-negative int".into()),
            _ => Err("pointer argument must be null or int".into()),
        },
        FfiTypeDesc::CString => match value {
            Value::Str(s) => {
                let buf = CString::new(s.as_str())
                    .map_err(|_| "str argument contains an embedded NUL byte".to_string())?;
                let ptr = buf.as_ptr();
                Ok(FfiArgStorage::CString(CStringArg { _buf: buf, ptr }))
            }
            _ => Err("str argument must be string".into()),
        },
    }
}

unsafe fn ffi_invoke(
    library_path: &str,
    symbol_name: &str,
    arg_types: &[FfiTypeDesc],
    ret_type: FfiTypeDesc,
    args: &[Value],
) -> Result<Value, String> {
    if arg_types.len() != args.len() {
        return Err(format!(
            "FFI argument count mismatch: expected {}, got {}",
            arg_types.len(),
            args.len()
        ));
    }

    let library = Library::new(library_path).map_err(|err| err.to_string())?;
    let symbol_buf = CString::new(symbol_name)
        .map_err(|_| "symbol name contains an embedded NUL byte".to_string())?;
    let symbol = library
        .get::<*mut c_void>(symbol_buf.as_bytes_with_nul())
        .map_err(|err| err.to_string())?;

    let mut prepared = Vec::with_capacity(args.len());
    for (expected, value) in arg_types.iter().zip(args.iter()) {
        prepared.push(build_ffi_arg(*expected, value)?);
    }

    let ffi_args: Vec<Arg> = prepared.iter().map(FfiArgStorage::as_arg).collect();
    let cif = Cif::new(
        arg_types.iter().map(FfiTypeDesc::ffi_type).collect::<Vec<_>>(),
        ret_type.ffi_type(),
    );
    let code_ptr = CodePtr(*symbol);

    match ret_type {
        FfiTypeDesc::Void => {
            let _: () = cif.call(code_ptr, &ffi_args);
            Ok(Value::Null)
        }
        FfiTypeDesc::Bool => Ok(Value::Bool(cif.call::<u8>(code_ptr, &ffi_args) != 0)),
        FfiTypeDesc::I8 => Ok(Value::Int(cif.call::<i8>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::U8 => Ok(Value::Int(cif.call::<u8>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::I16 => Ok(Value::Int(cif.call::<i16>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::U16 => Ok(Value::Int(cif.call::<u16>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::I32 => Ok(Value::Int(cif.call::<i32>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::U32 => Ok(Value::Int(cif.call::<u32>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::I64 => Ok(Value::Int(cif.call::<i64>(code_ptr, &ffi_args))),
        FfiTypeDesc::U64 => Ok(Value::Int(cif.call::<u64>(code_ptr, &ffi_args) as i64)),
        FfiTypeDesc::Isize => {
            if mem::size_of::<isize>() == 8 {
                Ok(Value::Int(cif.call::<i64>(code_ptr, &ffi_args)))
            } else {
                Ok(Value::Int(cif.call::<i32>(code_ptr, &ffi_args) as i64))
            }
        }
        FfiTypeDesc::Usize => {
            if mem::size_of::<usize>() == 8 {
                Ok(Value::Int(cif.call::<u64>(code_ptr, &ffi_args) as i64))
            } else {
                Ok(Value::Int(cif.call::<u32>(code_ptr, &ffi_args) as i64))
            }
        }
        FfiTypeDesc::F32 => Ok(Value::Float(cif.call::<f32>(code_ptr, &ffi_args) as f64)),
        FfiTypeDesc::F64 => Ok(Value::Float(cif.call::<f64>(code_ptr, &ffi_args))),
        FfiTypeDesc::Pointer => Ok(Value::Int(
            cif.call::<*const c_void>(code_ptr, &ffi_args) as usize as i64,
        )),
        FfiTypeDesc::CString => {
            let ptr = cif.call::<*const c_char>(code_ptr, &ffi_args);
            if ptr.is_null() {
                Ok(Value::Null)
            } else {
                Ok(Value::Str(Arc::new(
                    CStr::from_ptr(ptr).to_string_lossy().into_owned(),
                )))
            }
        }
    }
}

pub fn builtin_ffi_open(_vm: &mut VM, args: Vec<Value>) -> Value {
    let path = match args.get(0) {
        Some(value) => match ffi_library_path(value) {
            Ok(path) => path,
            Err(err) => return Value::Error(Arc::new(err)),
        },
        None => return Value::Error(Arc::new("ffi.open expects a library path".into())),
    };

    let resolved_path = resolve_library_path(_vm, &path);

    match validate_ffi_library(&resolved_path) {
        Ok(()) => Value::FfiLibrary(Arc::new(resolved_path)),
        Err(err) => Value::Error(Arc::new(err)),
    }
}

pub fn builtin_ffi_open_any(vm: &mut VM, args: Vec<Value>) -> Value {
    let candidates = match args.get(0) {
        Some(value) => match ffi_list(value, "paths") {
            Ok(paths) => paths,
            Err(err) => return Value::Error(Arc::new(err)),
        },
        None => return Value::Error(Arc::new("ffi.open_any expects a list of paths".into())),
    };

    let mut failures = Vec::new();
    for candidate in candidates {
        let raw = match ffi_string(&candidate, "paths entry") {
            Ok(path) => path,
            Err(err) => return Value::Error(Arc::new(err)),
        };
        let resolved = resolve_library_path(vm, &raw);
        match validate_ffi_library(&resolved) {
            Ok(()) => return Value::FfiLibrary(Arc::new(resolved)),
            Err(err) => failures.push(format!("{raw}: {err}")),
        }
    }

    Value::Error(Arc::new(format!(
        "failed to open any library candidate ({})",
        failures.join("; ")
    )))
}

pub fn builtin_ffi_close(_vm: &mut VM, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::FfiLibrary(_)) | Some(Value::Str(_)) => Value::Null,
        Some(_) => Value::Error(Arc::new("ffi.close expects an ffi handle or string path".into())),
        None => Value::Error(Arc::new("ffi.close expects an ffi handle or string path".into())),
    }
}

pub fn builtin_ffi_default_c(_vm: &mut VM, _args: Vec<Value>) -> Value {
    let path = match default_c_library_name() {
        Ok(path) => path,
        Err(err) => return Value::Error(Arc::new(err)),
    };

    match validate_ffi_library(&path) {
        Ok(()) => Value::FfiLibrary(Arc::new(path)),
        Err(err) => Value::Error(Arc::new(err)),
    }
}

pub fn builtin_ffi_default_c_path(_vm: &mut VM, _args: Vec<Value>) -> Value {
    match default_c_library_name() {
        Ok(path) => Value::Str(Arc::new(path)),
        Err(err) => Value::Error(Arc::new(err)),
    }
}

pub fn builtin_ffi_library_name(_vm: &mut VM, args: Vec<Value>) -> Value {
    let base = match args.get(0) {
        Some(value) => match ffi_string(value, "base") {
            Ok(base) => base,
            Err(err) => return Value::Error(Arc::new(err)),
        },
        None => return Value::Error(Arc::new("ffi.library_name expects a base name".into())),
    };

    match platform_library_name(&base) {
        Ok(name) => Value::Str(Arc::new(name)),
        Err(err) => Value::Error(Arc::new(err)),
    }
}

pub fn builtin_ffi_call(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() != 5 {
        return Value::Error(Arc::new(
            "ffi.call expects library, symbol, arg_types, ret_type, args".into(),
        ));
    }

    let library_path = match ffi_library_path(&args[0]) {
        Ok(path) => path,
        Err(err) => return Value::Error(Arc::new(err)),
    };
    let library_path = match &args[0] {
        Value::FfiLibrary(_) => library_path,
        _ => resolve_library_path(_vm, &library_path),
    };
    let symbol_name = match ffi_string(&args[1], "symbol") {
        Ok(symbol) => symbol,
        Err(err) => return Value::Error(Arc::new(err)),
    };
    let arg_types = match parse_ffi_type_list(&args[2], "arg_types") {
        Ok(types) => types,
        Err(err) => return Value::Error(Arc::new(err)),
    };
    let ret_type = match ffi_string(&args[3], "ret_type")
        .and_then(|name| FfiTypeDesc::from_name(&name))
    {
        Ok(ret) => ret,
        Err(err) => return Value::Error(Arc::new(err)),
    };
    let call_args = match ffi_list(&args[4], "args") {
        Ok(args) => args,
        Err(err) => return Value::Error(Arc::new(err)),
    };

    match unsafe { ffi_invoke(&library_path, &symbol_name, &arg_types, ret_type, &call_args) } {
        Ok(value) => value,
        Err(err) => Value::Error(Arc::new(err)),
    }
}

pub fn builtin_os_args(vm: &mut VM, _args: Vec<Value>) -> Value {
    let mut list = Vec::new();
    for arg in vm.script_args() {
        list.push(Value::Str(Arc::new(arg.clone())));
    }
    Value::List(Arc::new(Mutex::new(list)))
}

pub fn builtin_os_exit(_vm: &mut VM, args: Vec<Value>) -> Value {
    let code = match args.get(0) {
        Some(Value::Int(n)) => *n as i32,
        _ => 0,
    };
    std::process::exit(code)
}

pub fn builtin_path_join(_vm: &mut VM, args: Vec<Value>) -> Value {
    let parts = match args.get(0) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Str(Arc::new(String::new())),
    };
    let mut path = PathBuf::new();
    for part in parts {
        if let Value::Str(s) = part {
            path.push(s.as_str());
        }
    }
    Value::Str(Arc::new(path.to_string_lossy().to_string()))
}

pub fn builtin_process_run(_vm: &mut VM, args: Vec<Value>) -> Value {
    let cmd = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("cmd must be string".into())),
    };
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &cmd]).output()
    } else {
        Command::new("sh").args(["-c", &cmd]).output()
    };
    match output {
        Ok(out) => {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                Value::Str(Arc::new(s))
            } else {
                let s = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Value::Error(Arc::new(s))
            }
        }
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_regex_matches(_vm: &mut VM, args: Vec<Value>) -> Value {
    let pattern = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    match regex::Regex::new(pattern) {
        Ok(re) => Value::Bool(re.is_match(s)),
        Err(_) => Value::Bool(false),
    }
}

pub fn builtin_regex_find(_vm: &mut VM, args: Vec<Value>) -> Value {
    let pattern = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("pattern must be string".into())),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("input must be string".into())),
    };
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Value::Error(Arc::new(e.to_string())),
    };
    match re.find(s) {
        Some(m) => Value::Str(Arc::new(m.as_str().to_string())),
        None => Value::Error(Arc::new("no match".into())),
    }
}

pub fn builtin_regex_find_all(_vm: &mut VM, args: Vec<Value>) -> Value {
    let pattern = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("pattern must be string".into())),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("input must be string".into())),
    };
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Value::Error(Arc::new(e.to_string())),
    };
    let mut out = Vec::new();
    for m in re.find_iter(s) {
        out.push(Value::Str(Arc::new(m.as_str().to_string())));
    }
    Value::List(Arc::new(Mutex::new(out)))
}

pub fn builtin_regex_replace(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new(
            "replace expects pattern, input, replacement".into(),
        ));
    }
    let pattern = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("pattern must be string".into())),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("input must be string".into())),
    };
    let replacement = match args.get(2) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("replacement must be string".into())),
    };
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Value::Error(Arc::new(e.to_string())),
    };
    let out = re.replacen(s, 1, replacement).to_string();
    Value::Str(Arc::new(out))
}

pub fn builtin_regex_replace_all(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new(
            "replace_all expects pattern, input, replacement".into(),
        ));
    }
    let pattern = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("pattern must be string".into())),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("input must be string".into())),
    };
    let replacement = match args.get(2) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("replacement must be string".into())),
    };
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Value::Error(Arc::new(e.to_string())),
    };
    let out = re.replace_all(s, replacement).to_string();
    Value::Str(Arc::new(out))
}

pub fn builtin_regex_split(_vm: &mut VM, args: Vec<Value>) -> Value {
    let pattern = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("pattern must be string".into())),
    };
    let s = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("input must be string".into())),
    };
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Value::Error(Arc::new(e.to_string())),
    };
    let mut out = Vec::new();
    for part in re.split(s) {
        out.push(Value::Str(Arc::new(part.to_string())));
    }
    Value::List(Arc::new(Mutex::new(out)))
}

pub fn builtin_json_parse(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("input must be string".into())),
    };
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Object(map)) => json_to_value(serde_json::Value::Object(map)),
        Ok(_) => Value::Error(Arc::new("expected JSON object".into())),
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

fn json_to_value(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Error(Arc::new("unsupported JSON number".into()))
            }
        }
        serde_json::Value::String(s) => Value::Str(Arc::new(s)),
        serde_json::Value::Array(items) => Value::List(Arc::new(Mutex::new(
            items.into_iter().map(json_to_value).collect(),
        ))),
        serde_json::Value::Object(map) => {
            let mut out = std::collections::HashMap::new();
            for (k, v) in map {
                out.insert(k, json_to_value(v));
            }
            Value::Map(Arc::new(Mutex::new(out)))
        }
    }
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::Value::Number((*n).into()),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(f.to_string())),
        Value::Str(s) => serde_json::Value::String(s.to_string()),
        Value::List(l) => {
            let list = l.lock().unwrap();
            serde_json::Value::Array(list.iter().map(value_to_json).collect())
        }
        Value::Map(m) => {
            let map = m.lock().unwrap();
            let mut out = serde_json::Map::new();
            for (k, v) in map.iter() {
                out.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(out)
        }
        other => serde_json::Value::String(other.stringify()),
    }
}

pub fn builtin_json_stringify(_vm: &mut VM, args: Vec<Value>) -> Value {
    let map = match args.get(0) {
        Some(Value::Map(m)) => m.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("input must be map".into())),
    };
    let mut out = serde_json::Map::new();
    for (k, v) in map.iter() {
        out.insert(k.clone(), value_to_json(v));
    }
    match serde_json::to_string(&serde_json::Value::Object(out)) {
        Ok(s) => Value::Str(Arc::new(s)),
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_json_pretty(_vm: &mut VM, args: Vec<Value>) -> Value {
    let map = match args.get(0) {
        Some(Value::Map(m)) => m.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("input must be map".into())),
    };
    let mut out = serde_json::Map::new();
    for (k, v) in map.iter() {
        out.insert(k.clone(), value_to_json(v));
    }
    match serde_json::to_string_pretty(&serde_json::Value::Object(out)) {
        Ok(s) => Value::Str(Arc::new(s)),
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_net_http_get(_vm: &mut VM, args: Vec<Value>) -> Value {
    let url = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("url must be string".into())),
    };
    match reqwest::blocking::get(url) {
        Ok(resp) => match resp.text() {
            Ok(text) => Value::Str(Arc::new(text)),
            Err(e) => Value::Error(Arc::new(e.to_string())),
        },
        Err(e) => Value::Error(Arc::new(e.to_string())),
    }
}

pub fn builtin_map_has(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    let map = match &args[0] {
        Value::Map(m) => m.lock().unwrap(),
        _ => return Value::Bool(false),
    };
    let key = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => return Value::Bool(false),
    };
    Value::Bool(map.contains_key(key))
}

pub fn map_keys(_vm: &mut VM, args: Vec<Value>) -> Value {
    let map = match args.get(0) {
        Some(Value::Map(m)) => m.lock().unwrap(),
        _ => return Value::List(Arc::new(Mutex::new(Vec::new()))),
    };
    let mut keys = Vec::new();
    for k in map.keys() {
        keys.push(Value::Str(Arc::new(k.clone())));
    }
    Value::List(Arc::new(Mutex::new(keys)))
}

pub fn builtin_map_values(_vm: &mut VM, args: Vec<Value>) -> Value {
    let map = match args.get(0) {
        Some(Value::Map(m)) => m.lock().unwrap(),
        _ => return Value::List(Arc::new(Mutex::new(Vec::new()))),
    };
    let mut values = Vec::new();
    for v in map.values() {
        values.push(v.clone());
    }
    Value::List(Arc::new(Mutex::new(values)))
}

pub fn builtin_list_push(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("push expects list and value".into()));
    }
    let list = match args.get(0) {
        Some(Value::List(l)) => l,
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let value = args.get(1).cloned().unwrap_or(Value::Null);
    list.lock().unwrap().push(value);
    Value::Null
}

pub fn builtin_list_pop(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list = match args.get(0) {
        Some(Value::List(l)) => l,
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let mut borrow = list.lock().unwrap();
    match borrow.pop() {
        Some(v) => v,
        None => Value::Error(Arc::new("pop from empty list".into())),
    }
}

pub fn builtin_list_first(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list = match args.get(0) {
        Some(Value::List(l)) => l,
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let borrow = list.lock().unwrap();
    match borrow.first() {
        Some(v) => v.clone(),
        None => Value::Error(Arc::new("empty list".into())),
    }
}

pub fn builtin_list_last(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list = match args.get(0) {
        Some(Value::List(l)) => l,
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let borrow = list.lock().unwrap();
    match borrow.last() {
        Some(v) => v.clone(),
        None => Value::Error(Arc::new("empty list".into())),
    }
}

pub fn builtin_list_insert(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("insert expects list, index, value".into()));
    }
    let list = match args.get(0) {
        Some(Value::List(l)) => l,
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let idx = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("index must be int".into())),
    };
    let value = args.get(2).cloned().unwrap_or(Value::Null);
    let mut borrow = list.lock().unwrap();
    if idx < 0 || idx as usize > borrow.len() {
        return Value::Error(Arc::new("index out of bounds".into()));
    }
    borrow.insert(idx as usize, value);
    Value::Null
}

pub fn builtin_list_remove(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("remove expects list and index".into()));
    }
    let list = match args.get(0) {
        Some(Value::List(l)) => l,
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let idx = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("index must be int".into())),
    };
    let mut borrow = list.lock().unwrap();
    if idx < 0 || idx as usize >= borrow.len() {
        return Value::Error(Arc::new("index out of bounds".into()));
    }
    borrow.remove(idx as usize)
}

pub fn builtin_list_slice(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("slice expects list, start, end".into()));
    }
    let list = match args.get(0) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let start = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("start must be int".into())),
    };
    let end = match args.get(2) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("end must be int".into())),
    };
    let len = list.len() as i64;
    let mut s = if start < 0 { len + start } else { start };
    let mut e = if end < 0 { len + end } else { end };
    s = s.clamp(0, len);
    e = e.clamp(0, len);
    if e < s {
        e = s;
    }
    let mut out = Vec::new();
    for i in s..e {
        out.push(list[i as usize].clone());
    }
    Value::List(Arc::new(Mutex::new(out)))
}

pub fn builtin_list_reverse(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list = match args.get(0) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    let mut out = list;
    out.reverse();
    Value::List(Arc::new(Mutex::new(out)))
}

pub fn builtin_list_sort(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list = match args.get(0) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("first arg must be list".into())),
    };
    if list.iter().all(|v| matches!(v, Value::Int(_))) {
        let mut out = list;
        out.sort_by(|a, b| match (a, b) {
            (Value::Int(x), Value::Int(y)) => x.cmp(y),
            _ => Ordering::Equal,
        });
        return Value::List(Arc::new(Mutex::new(out)));
    }
    if list
        .iter()
        .all(|v| matches!(v, Value::Int(_) | Value::Float(_)))
    {
        let mut out = list;
        out.sort_by(|a, b| {
            let ax = match a {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => 0.0,
            };
            let bx = match b {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => 0.0,
            };
            ax.partial_cmp(&bx).unwrap_or(Ordering::Equal)
        });
        return Value::List(Arc::new(Mutex::new(out)));
    }
    if list.iter().all(|v| matches!(v, Value::Str(_))) {
        let mut out = list;
        out.sort_by(|a, b| match (a, b) {
            (Value::Str(x), Value::Str(y)) => x.cmp(y),
            _ => Ordering::Equal,
        });
        return Value::List(Arc::new(Mutex::new(out)));
    }
    Value::Error(Arc::new(
        "sort supports only int, float, or str lists".into(),
    ))
}

pub fn builtin_str_split(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("split expects string and delimiter".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("first arg must be string".into())),
    };
    let delim = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("delimiter must be string".into())),
    };
    let mut out = Vec::new();
    if delim.is_empty() {
        for ch in s.chars() {
            out.push(Value::Str(Arc::new(ch.to_string())));
        }
    } else {
        for part in s.split(&delim) {
            out.push(Value::Str(Arc::new(part.to_string())));
        }
    }
    Value::List(Arc::new(Mutex::new(out)))
}

pub fn builtin_str_join(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("join expects parts and separator".into()));
    }
    let parts = match args.get(0) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("parts must be list".into())),
    };
    let sep = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("separator must be string".into())),
    };
    let mut out = String::new();
    for (i, v) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push_str(&sep);
        }
        out.push_str(&v.stringify());
    }
    Value::Str(Arc::new(out))
}

pub fn builtin_str_trim(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    Value::Str(Arc::new(s.trim().to_string()))
}

pub fn builtin_str_trim_start(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    Value::Str(Arc::new(s.trim_start().to_string()))
}

pub fn builtin_str_trim_end(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    Value::Str(Arc::new(s.trim_end().to_string()))
}

pub fn builtin_str_upper(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    Value::Str(Arc::new(s.to_uppercase()))
}

pub fn builtin_str_lower(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    Value::Str(Arc::new(s.to_lowercase()))
}

pub fn builtin_str_starts_with(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    let prefix = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    Value::Bool(s.starts_with(prefix))
}

pub fn builtin_str_ends_with(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    let suffix = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    Value::Bool(s.ends_with(suffix))
}

pub fn builtin_str_replace(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("replace expects s, old, new".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("s must be string".into())),
    };
    let old = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("old must be string".into())),
    };
    let new = match args.get(2) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("new must be string".into())),
    };
    Value::Str(Arc::new(s.replacen(&old, &new, 1)))
}

pub fn builtin_str_replace_all(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("replace_all expects s, old, new".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("s must be string".into())),
    };
    let old = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("old must be string".into())),
    };
    let new = match args.get(2) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("new must be string".into())),
    };
    Value::Str(Arc::new(s.replace(&old, &new)))
}

pub fn builtin_str_count(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Int(0);
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Int(0),
    };
    let sub = match args.get(1) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Int(0),
    };
    if sub.is_empty() {
        return Value::Int(0);
    }
    Value::Int(s.matches(sub).count() as i64)
}

pub fn builtin_str_slice(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("slice expects s, start, end".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("s must be string".into())),
    };
    let start = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("start must be int".into())),
    };
    let end = match args.get(2) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("end must be int".into())),
    };
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;
    let mut s_idx = if start < 0 { len + start } else { start };
    let mut e_idx = if end < 0 { len + end } else { end };
    s_idx = s_idx.clamp(0, len);
    e_idx = e_idx.clamp(0, len);
    if e_idx < s_idx {
        e_idx = s_idx;
    }
    let out: String = chars[s_idx as usize..e_idx as usize].iter().collect();
    Value::Str(Arc::new(out))
}

pub fn builtin_str_repeat(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("repeat expects s, n".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("s must be string".into())),
    };
    let n = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("n must be int".into())),
    };
    if n < 0 {
        return Value::Error(Arc::new("n must be >= 0".into()));
    }
    Value::Str(Arc::new(s.repeat(n as usize)))
}

pub fn builtin_str_pad_left(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("pad_left expects s, width, char".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("s must be string".into())),
    };
    let width = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("width must be int".into())),
    };
    let ch = match args.get(2) {
        Some(Value::Str(s)) => s.chars().next(),
        _ => None,
    };
    let pad = match ch {
        Some(c) => c,
        None => return Value::Error(Arc::new("char must be non-empty string".into())),
    };
    let len = s.chars().count() as i64;
    if width <= len {
        return Value::Str(Arc::new(s));
    }
    let mut out = String::new();
    for _ in 0..(width - len) {
        out.push(pad);
    }
    out.push_str(&s);
    Value::Str(Arc::new(out))
}

pub fn builtin_str_pad_right(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("pad_right expects s, width, char".into()));
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("s must be string".into())),
    };
    let width = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("width must be int".into())),
    };
    let ch = match args.get(2) {
        Some(Value::Str(s)) => s.chars().next(),
        _ => None,
    };
    let pad = match ch {
        Some(c) => c,
        None => return Value::Error(Arc::new("char must be non-empty string".into())),
    };
    let len = s.chars().count() as i64;
    if width <= len {
        return Value::Str(Arc::new(s));
    }
    let mut out = s.clone();
    for _ in 0..(width - len) {
        out.push(pad);
    }
    Value::Str(Arc::new(out))
}

pub fn builtin_str_to_int(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    match s.parse::<i64>() {
        Ok(n) => Value::Int(n),
        Err(_) => Value::Error(Arc::new("invalid int".into())),
    }
}

pub fn builtin_str_to_float(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    match s.parse::<f64>() {
        Ok(n) => Value::Float(n),
        Err(_) => Value::Error(Arc::new("invalid float".into())),
    }
}

pub fn builtin_str_from_int(_vm: &mut VM, args: Vec<Value>) -> Value {
    let n = match args.get(0) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("arg must be int".into())),
    };
    Value::Str(Arc::new(n.to_string()))
}

pub fn builtin_str_from_float(_vm: &mut VM, args: Vec<Value>) -> Value {
    let n = match args.get(0) {
        Some(Value::Float(n)) => *n,
        Some(Value::Int(n)) => *n as f64,
        _ => return Value::Error(Arc::new("arg must be number".into())),
    };
    Value::Str(Arc::new(n.to_string()))
}

pub fn builtin_str_chars(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("arg must be string".into())),
    };
    let mut out = Vec::new();
    for ch in s.chars() {
        out.push(Value::Str(Arc::new(ch.to_string())));
    }
    Value::List(Arc::new(Mutex::new(out)))
}

pub fn builtin_str_len(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Int(0),
    };
    Value::Int(s.chars().count() as i64)
}

pub fn builtin_str_is_numeric(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

pub fn builtin_str_is_alpha(_vm: &mut VM, args: Vec<Value>) -> Value {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Bool(false),
    };
    Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()))
}

pub fn builtin_str_format(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("format expects template and args".into()));
    }
    let template = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Value::Error(Arc::new("template must be string".into())),
    };
    let list = match args.get(1) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("args must be list".into())),
    };
    let mut out = String::new();
    let mut iter = template.chars().peekable();
    let mut arg_index = 0usize;
    while let Some(ch) = iter.next() {
        if ch == '{' {
            if iter.peek() == Some(&'{') {
                iter.next();
                out.push('{');
                continue;
            }
            if iter.peek() == Some(&'}') {
                iter.next();
                if arg_index >= list.len() {
                    return Value::Error(Arc::new("not enough args for format".into()));
                }
                out.push_str(&list[arg_index].stringify());
                arg_index += 1;
                continue;
            }
        }
        if ch == '}' && iter.peek() == Some(&'}') {
            iter.next();
            out.push('}');
            continue;
        }
        out.push(ch);
    }
    Value::Str(Arc::new(out))
}

fn number_from_value(v: &Value) -> Option<(f64, bool)> {
    match v {
        Value::Int(n) => Some((*n as f64, true)),
        Value::Float(f) => Some((*f, false)),
        _ => None,
    }
}

pub fn builtin_math_pow(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("pow expects base and exp".into()));
    }
    let a = number_from_value(&args[0]).map(|v| v.0).unwrap_or(0.0);
    let b = number_from_value(&args[1]).map(|v| v.0).unwrap_or(0.0);
    Value::Float(a.powf(b))
}

pub fn builtin_math_sqrt(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    if x < 0.0 {
        return Value::Error(Arc::new("sqrt of negative".into()));
    }
    Value::Float(x.sqrt())
}

pub fn builtin_math_abs(_vm: &mut VM, args: Vec<Value>) -> Value {
    match args.get(0) {
        Some(Value::Int(n)) => Value::Int(n.abs()),
        Some(Value::Float(f)) => Value::Float(f.abs()),
        _ => Value::Error(Arc::new("abs expects number".into())),
    }
}

pub fn builtin_math_floor(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Value::Float(x.floor())
}

pub fn builtin_math_ceil(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Value::Float(x.ceil())
}

pub fn builtin_math_round(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Value::Float(x.round())
}

pub fn builtin_math_min(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("min expects two numbers".into()));
    }
    let (a, ai) = match number_from_value(&args[0]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("min expects numbers".into())),
    };
    let (b, bi) = match number_from_value(&args[1]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("min expects numbers".into())),
    };
    if ai && bi {
        Value::Int(a.min(b) as i64)
    } else {
        Value::Float(a.min(b))
    }
}

pub fn builtin_math_max(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("max expects two numbers".into()));
    }
    let (a, ai) = match number_from_value(&args[0]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("max expects numbers".into())),
    };
    let (b, bi) = match number_from_value(&args[1]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("max expects numbers".into())),
    };
    if ai && bi {
        Value::Int(a.max(b) as i64)
    } else {
        Value::Float(a.max(b))
    }
}

pub fn builtin_math_clamp(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 3 {
        return Value::Error(Arc::new("clamp expects x, lo, hi".into()));
    }
    let (x, xi) = match number_from_value(&args[0]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("clamp expects numbers".into())),
    };
    let (lo, _) = match number_from_value(&args[1]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("clamp expects numbers".into())),
    };
    let (hi, _) = match number_from_value(&args[2]) {
        Some(v) => v,
        None => return Value::Error(Arc::new("clamp expects numbers".into())),
    };
    let out = x.max(lo).min(hi);
    if xi {
        Value::Int(out as i64)
    } else {
        Value::Float(out)
    }
}

pub fn builtin_math_log(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    if x <= 0.0 {
        return Value::Error(Arc::new("log expects positive".into()));
    }
    Value::Float(x.ln())
}

pub fn builtin_math_log2(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    if x <= 0.0 {
        return Value::Error(Arc::new("log2 expects positive".into()));
    }
    Value::Float(x.log2())
}

pub fn builtin_math_sin(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Value::Float(x.sin())
}

pub fn builtin_math_cos(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Value::Float(x.cos())
}

pub fn builtin_math_tan(_vm: &mut VM, args: Vec<Value>) -> Value {
    let x = number_from_value(args.get(0).unwrap_or(&Value::Int(0)))
        .map(|v| v.0)
        .unwrap_or(0.0);
    Value::Float(x.tan())
}

pub fn builtin_math_random(_vm: &mut VM, _args: Vec<Value>) -> Value {
    let mut rng = rand::thread_rng();
    Value::Float(rng.gen::<f64>())
}

pub fn builtin_math_rand_int(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("rand_int expects lo and hi".into()));
    }
    let lo = match args.get(0) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("lo must be int".into())),
    };
    let hi = match args.get(1) {
        Some(Value::Int(n)) => *n,
        _ => return Value::Error(Arc::new("hi must be int".into())),
    };
    if lo > hi {
        return Value::Error(Arc::new("lo must be <= hi".into()));
    }
    let mut rng = rand::thread_rng();
    let n = rng.gen_range(lo..=hi);
    Value::Int(n)
}

pub fn load_module(vm: &mut VM, args: Vec<Value>) -> Value {
    let source = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Error(Arc::new("source must be string".into())),
    };
    match vm.load_module(&source) {
        Ok(m) => m,
        Err(e) => Value::Error(Arc::new(e)),
    }
}

// ── New builtins added for full stdlib coverage ────────────────────────────────

pub fn builtin_os_env(_vm: &mut VM, _args: Vec<Value>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in std::env::vars() {
        map.insert(k, Value::Str(Arc::new(v)));
    }
    Value::Map(Arc::new(Mutex::new(map)))
}

pub fn builtin_str_contains(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Bool(false),
    };
    let sub = match args.get(1) {
        Some(Value::Str(s)) => s.to_string(),
        _ => return Value::Bool(false),
    };
    Value::Bool(s.contains(&*sub))
}

pub fn builtin_list_contains(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Bool(false);
    }
    let list = match args.get(0) {
        Some(Value::List(l)) => l.lock().unwrap().clone(),
        _ => return Value::Bool(false),
    };
    let target = match args.get(1) {
        Some(v) => v.stringify(),
        None => return Value::Bool(false),
    };
    for item in &list {
        if item.stringify() == target { return Value::Bool(true); }
    }
    Value::Bool(false)
}

pub fn builtin_list_sort_inplace(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list_arc = match args.get(0) {
        Some(Value::List(l)) => l.clone(),
        _ => return Value::Error(Arc::new("sort expects a list".into())),
    };
    let mut list = list_arc.lock().unwrap();
    if list.iter().all(|v| matches!(v, Value::Int(_) | Value::Float(_))) {
        list.sort_by(|a, b| {
            let ax = match a { Value::Int(n) => *n as f64, Value::Float(f) => *f, _ => 0.0 };
            let bx = match b { Value::Int(n) => *n as f64, Value::Float(f) => *f, _ => 0.0 };
            ax.partial_cmp(&bx).unwrap_or(std::cmp::Ordering::Equal)
        });
        Value::Null
    } else if list.iter().all(|v| matches!(v, Value::Str(_))) {
        list.sort_by(|a, b| match (a, b) {
            (Value::Str(x), Value::Str(y)) => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        });
        Value::Null
    } else {
        Value::Error(Arc::new("sort supports int, float, or str lists".into()))
    }
}

pub fn builtin_list_reverse_inplace(_vm: &mut VM, args: Vec<Value>) -> Value {
    let list_arc = match args.get(0) {
        Some(Value::List(l)) => l.clone(),
        _ => return Value::Error(Arc::new("reverse expects a list".into())),
    };
    list_arc.lock().unwrap().reverse();
    Value::Null
}

pub fn builtin_map_merge(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("merge expects two maps".into()));
    }
    let base = match args.get(0) {
        Some(Value::Map(m)) => m.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("first arg must be map".into())),
    };
    let overrides = match args.get(1) {
        Some(Value::Map(m)) => m.lock().unwrap().clone(),
        _ => return Value::Error(Arc::new("second arg must be map".into())),
    };
    let mut merged = base;
    for (k, v) in overrides {
        merged.insert(k, v);
    }
    Value::Map(Arc::new(Mutex::new(merged)))
}

pub fn builtin_math_div(_vm: &mut VM, args: Vec<Value>) -> Value {
    if args.len() < 2 {
        return Value::Error(Arc::new("div expects two numbers".into()));
    }
    let a = match args.get(0) {
        Some(Value::Int(n))   => *n as f64,
        Some(Value::Float(f)) => *f,
        _ => return Value::Error(Arc::new("a must be number".into())),
    };
    let b = match args.get(1) {
        Some(Value::Int(n))   => *n as f64,
        Some(Value::Float(f)) => *f,
        _ => return Value::Error(Arc::new("b must be number".into())),
    };
    if b == 0.0 { return Value::Error(Arc::new("division by zero".into())); }
    Value::Float(a / b)
}
