//! The MatchBox plugin: registers `yamlDeserialize`, `yamlDeserializeFile` and `yamlSerialize` with the same names and
//! arguments as the JVM module, and moves values between the VM and the YAML core.

use crate::model::{DateTimeValue, Failure, Kind, Value as Model};
use crate::{read, write};
use matchbox_plugin_sdk::{
    Call, CollectionsV1, Context, DatetimeV1, ErrorsV1, HostV1, IntegersV1, Invoke, Kind as VmKind, PathsV1, StructsV1, Text, TypesV1,
    Value, ValuesV1, guarded, require_collections, require_datetime, require_errors, require_integers, require_paths, require_structs,
    require_types, require_values,
};
use std::{ffi::c_void, sync::OnceLock};

/// The capabilities the host granted at initialization.
struct Apis {
    types: &'static TypesV1,
    collections: &'static CollectionsV1,
    integers: &'static IntegersV1,
    structs: &'static StructsV1,
    datetime: &'static DatetimeV1,
    paths: &'static PathsV1,
    errors: &'static ErrorsV1,
}

static APIS: OnceLock<Apis> = OnceLock::new();

fn apis() -> Result<&'static Apis, String> {
    APIS.get().ok_or_else(|| "The YAML plugin was not initialized".to_string())
}

type Outcome<T> = std::result::Result<T, Failure>;

fn plain(error: String) -> Failure {
    Failure::new(Kind::Serialization, error)
}

/// A value read from the VM, ready to write as YAML.
fn from_vm(call: &Call<'_>, a: &Apis, value: Value) -> Outcome<Model> {
    Ok(match call.kind(a.types, value).map_err(plain)? {
        VmKind::Null => Model::Null,
        VmKind::Bool => Model::Bool(call.boolean(a.types, value).map_err(plain)?),
        VmKind::Number => number(call.number(value).map_err(plain)?),
        VmKind::String => Model::Str(call.string(value).map_err(plain)?),
        VmKind::Array => {
            let length = call.array_len(a.collections, value).map_err(plain)?;
            let mut items = Vec::with_capacity(length);
            for index in 0..length {
                items.push(from_vm(call, a, call.array_get(a.collections, value, index).map_err(plain)?)?);
            }
            Model::Array(items)
        }
        VmKind::Struct => {
            let mut entries = Vec::new();
            for key in call.struct_keys(a.collections, value).map_err(plain)? {
                let inner = match call.struct_get(a.collections, value, &key).map_err(plain)? {
                    Some(inner) => inner,
                    None => call.null_new().map_err(plain)?,
                };
                entries.push((key, from_vm(call, a, inner)?));
            }
            Model::Table(entries)
        }
        VmKind::Binary => return Err(plain("Cannot serialize a binary value to YAML".into())),
        VmKind::Other => {
            if call.is_datetime(a.datetime, value).map_err(plain)? {
                let (seconds, nanos, zone) = call.datetime_get(a.datetime, value).map_err(plain)?;
                Model::DateTime(DateTimeValue { seconds, nanos, zone })
            } else {
                return Err(plain("Cannot serialize a value of this type to YAML".into()));
            }
        }
    })
}

/// MatchBox numbers are not told apart as integers and decimals, so a whole number is a YAML integer and any other a float.
fn number(n: f64) -> Model {
    if n.is_finite() && n.fract() == 0.0 && n >= -9_223_372_036_854_775_808.0 && n < 9_223_372_036_854_775_808.0 {
        Model::Int(n as i64)
    } else {
        Model::Float(n)
    }
}

/// A parsed value as a VM value. Mappings keep the order of the document and are case-insensitive, like the JVM's structs.
fn to_vm(call: &Call<'_>, a: &Apis, value: &Model) -> Outcome<Value> {
    Ok(match value {
        Model::Null => call.null_new(),
        Model::Bool(b) => call.bool_new(*b),
        Model::Int(i) => call.int_new(a.integers, *i),
        Model::Float(f) => call.number_new(*f),
        Model::Str(s) => call.string_new(s),
        Model::DateTime(dt) => call.datetime_new(a.datetime, dt.seconds, dt.nanos, &dt.zone),
        Model::Array(items) => {
            let array = call.array_new(a.collections).map_err(plain)?;
            for item in items {
                let converted = to_vm(call, a, item)?;
                call.array_push(a.collections, array, converted).map_err(plain)?;
            }
            Ok(array)
        }
        Model::Table(entries) => {
            let table = call.struct_new_with(a.structs, true, false).map_err(plain)?;
            for (key, inner) in entries {
                let converted = to_vm(call, a, inner)?;
                call.struct_set(table, key, converted).map_err(plain)?;
            }
            Ok(table)
        }
    }
    .map_err(plain)?)
}

/// The native backend reads and writes UTF-8 only.
fn check_charset(call: &Call<'_>, a: &Apis, index: usize) -> Outcome<()> {
    let value = call.argument(index).map_err(plain)?;
    if call.kind(a.types, value).map_err(plain)? == VmKind::Null {
        return Ok(());
    }
    let name = call.string(value).map_err(plain)?;
    match name.to_ascii_lowercase().replace('_', "-").as_str() {
        "utf-8" | "utf8" | "" => Ok(()),
        _ => Err(Failure::new(Kind::Io, format!("Unsupported charset: {name}. The native YAML backend reads and writes UTF-8"))),
    }
}

fn text_argument(call: &Call<'_>, a: &Apis, index: usize) -> Outcome<Option<String>> {
    let value = call.argument(index).map_err(plain)?;
    if call.kind(a.types, value).map_err(plain)? == VmKind::Null {
        return Ok(None);
    }
    Ok(Some(call.string(value).map_err(plain)?))
}

fn resolve_path(call: &Call<'_>, a: &Apis, path: &str) -> Outcome<String> {
    call.expand_path(a.paths, path).map_err(|e| Failure::new(Kind::Io, e))
}

// yamlDeserialize( content )
fn yaml_deserialize(call: &Call<'_>, a: &Apis) -> Outcome<Value> {
    let text = call.string(call.argument(0).map_err(plain)?).map_err(plain)?;
    to_vm(call, a, &read::read(&text)?)
}

// yamlDeserializeFile( filepath, charset )
fn yaml_deserialize_file(call: &Call<'_>, a: &Apis) -> Outcome<Value> {
    let path = call.string(call.argument(0).map_err(plain)?).map_err(plain)?;
    check_charset(call, a, 1)?;
    let resolved = resolve_path(call, a, &path)?;
    let text = std::fs::read_to_string(&resolved).map_err(|_| Failure::new(Kind::Io, "Error deserializing yaml"))?;
    to_vm(call, a, &read::read(&text)?)
}

// yamlSerialize( content, filepath, charset ): the text, or null when it was written to filepath
fn yaml_serialize(call: &Call<'_>, a: &Apis) -> Outcome<Value> {
    let content = call.argument(0).map_err(plain)?;
    let file = text_argument(call, a, 1)?;
    check_charset(call, a, 2)?;
    let text = write::write(&from_vm(call, a, content)?)?;
    match file {
        None => call.string_new(&text).map_err(plain),
        Some(path) => {
            let resolved = resolve_path(call, a, &path)?;
            std::fs::write(&resolved, text).map_err(|_| Failure::new(Kind::Io, "Error serializing yaml"))?;
            call.null_new().map_err(plain)
        }
    }
}

/// Runs a function body, turning its failure into the BoxLang exception the JVM module throws.
unsafe fn run(
    ctx: Context,
    api: *const ValuesV1,
    args: *const Value,
    len: usize,
    out: *mut Value,
    body: fn(&Call<'_>, &Apis) -> Outcome<Value>,
) -> i32 {
    unsafe {
        guarded(ctx, api, out, || {
            let call = Call::new(ctx, api, args, len);
            let a = apis()?;
            body(&call, a).map_err(|failure| match failure.kind.exception_type() {
                Some(exception_type) => call.error_typed(a.errors, exception_type, &failure.message, ""),
                None => failure.message,
            })
        })
    }
}

macro_rules! entry {
    ($name:ident, $body:path) => {
        unsafe extern "C" fn $name(
            ctx: Context,
            api: *const ValuesV1,
            _: *mut c_void,
            args: *const Value,
            len: usize,
            out: *mut Value,
        ) -> i32 {
            unsafe { run(ctx, api, args, len, out, $body) }
        }
    };
}
entry!(deserialize_entry, yaml_deserialize);
entry!(deserialize_file_entry, yaml_deserialize_file);
entry!(serialize_entry, yaml_serialize);

/// Initializes the plugin: requires the capabilities, then registers the three functions.
///
/// # Safety
/// Called by the MatchBox host with its own pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn matchbox_plugin_init_v1(host: *const HostV1, ctx: Context) -> i32 {
    macro_rules! need {
        ($call:expr) => {
            match unsafe { $call } {
                Ok(api) => api,
                Err(status) => return status,
            }
        };
    }
    if unsafe { require_values(host, ctx) }.is_err() {
        return 1;
    }
    let granted = Apis {
        types: need!(require_types(host, ctx)),
        collections: need!(require_collections(host, ctx)),
        integers: need!(require_integers(host, ctx)),
        structs: need!(require_structs(host, ctx)),
        datetime: need!(require_datetime(host, ctx)),
        paths: need!(require_paths(host, ctx)),
        errors: need!(require_errors(host, ctx)),
    };
    let _ = APIS.set(granted);
    let host = unsafe { &*host };
    let functions: [(&str, Invoke); 3] = [
        (r#"{"name":"yamlDeserialize","arguments":[{"name":"content","required":true}]}"#, deserialize_entry),
        (
            r#"{"name":"yamlDeserializeFile","arguments":[{"name":"filepath","required":true},{"name":"charset","default":"UTF-8"}]}"#,
            deserialize_file_entry,
        ),
        (
            r#"{"name":"yamlSerialize","arguments":[{"name":"content","required":true},{"name":"filepath","default":null},{"name":"charset","default":"UTF-8"}]}"#,
            serialize_entry,
        ),
    ];
    for (metadata, invoke) in functions {
        if unsafe { (host.register)(ctx, Text::new(metadata), invoke) } != 0 {
            return 1;
        }
    }
    0
}
