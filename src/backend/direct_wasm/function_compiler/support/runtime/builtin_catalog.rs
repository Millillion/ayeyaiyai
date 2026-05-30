use super::*;

const BOUND_FUNCTION_PROTOTYPE_CALL_PREFIX: &str = "__ayy_bound_call__";
const BUILTIN_RUNTIME_FUNCTION_NAMES: &[&str] = &[
    "Math.abs",
    "Math.acos",
    "Math.asin",
    "Math.atan",
    "Math.atan2",
    "Math.ceil",
    "Math.cos",
    "Math.exp",
    "Math.floor",
    "Math.log",
    "Math.max",
    "Math.min",
    "Math.pow",
    "Math.random",
    "Math.round",
    "Math.sin",
    "Math.sqrt",
    "Math.tan",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "Uint8Array",
    "Int8Array",
    "Uint16Array",
    "Int16Array",
    "Uint32Array",
    "Int32Array",
    "Float32Array",
    "Float64Array",
    "Uint8ClampedArray",
    "BigInt64Array",
    "BigUint64Array",
];

pub(in crate::backend::direct_wasm) fn builtin_prototype_function_name(
    object_name: &str,
    property_name: &str,
) -> Option<&'static str> {
    if property_name == "constructor" && builtin_constructor_prototype_kind(object_name).is_some() {
        return Some(match object_name {
            "Object" => "Object",
            "Array" => "Array",
            "ArrayBuffer" => "ArrayBuffer",
            "SharedArrayBuffer" => "SharedArrayBuffer",
            "DataView" => "DataView",
            "Date" => "Date",
            "RegExp" => "RegExp",
            "Map" => "Map",
            "Set" => "Set",
            "Error" => "Error",
            "EvalError" => "EvalError",
            "RangeError" => "RangeError",
            "ReferenceError" => "ReferenceError",
            "SyntaxError" => "SyntaxError",
            "TypeError" => "TypeError",
            "URIError" => "URIError",
            "AggregateError" => "AggregateError",
            "SuppressedError" => "SuppressedError",
            "Promise" => "Promise",
            "WeakMap" => "WeakMap",
            "WeakRef" => "WeakRef",
            "WeakSet" => "WeakSet",
            "Uint8Array" => "Uint8Array",
            "Int8Array" => "Int8Array",
            "Uint16Array" => "Uint16Array",
            "Int16Array" => "Int16Array",
            "Uint32Array" => "Uint32Array",
            "Int32Array" => "Int32Array",
            "Float32Array" => "Float32Array",
            "Float64Array" => "Float64Array",
            "Uint8ClampedArray" => "Uint8ClampedArray",
            "BigInt64Array" => "BigInt64Array",
            "BigUint64Array" => "BigUint64Array",
            "Number" => "Number",
            "String" => "String",
            "Boolean" => "Boolean",
            "BigInt" => "BigInt",
            "Symbol" => "Symbol",
            "Function" => "Function",
            _ => return None,
        });
    }

    match (object_name, property_name) {
        ("Function", "call") => Some("Function.prototype.call"),
        ("Function", "apply") => Some("Function.prototype.apply"),
        ("Function", "bind") => Some("Function.prototype.bind"),
        ("Array", "join") => Some("Array.prototype.join"),
        ("Array", "toString") => Some("Array.prototype.toString"),
        ("Array", "push") => Some("Array.prototype.push"),
        ("Array", "reverse") => Some("Array.prototype.reverse"),
        ("Array", "sort") => Some("Array.prototype.sort"),
        ("Object", "hasOwnProperty") => Some("Object.prototype.hasOwnProperty"),
        ("Object", "propertyIsEnumerable") => Some("Object.prototype.propertyIsEnumerable"),
        ("Object", "toString") => Some("Object.prototype.toString"),
        ("Object", "valueOf") => Some("Object.prototype.valueOf"),
        ("Object", "__lookupGetter__") => Some("Object.prototype.__lookupGetter__"),
        ("Object", "__lookupSetter__") => Some("Object.prototype.__lookupSetter__"),
        ("Date", "toString") => Some("Date.prototype.toString"),
        ("Date", "valueOf") => Some("Date.prototype.valueOf"),
        ("Date", "getTime") => Some("Date.prototype.getTime"),
        ("Date", "getFullYear") => Some("Date.prototype.getFullYear"),
        ("Date", "getUTCFullYear") => Some("Date.prototype.getUTCFullYear"),
        ("Date", "getMonth") => Some("Date.prototype.getMonth"),
        ("Date", "getUTCMonth") => Some("Date.prototype.getUTCMonth"),
        ("Date", "getDate") => Some("Date.prototype.getDate"),
        ("Date", "getUTCDate") => Some("Date.prototype.getUTCDate"),
        ("Date", "getDay") => Some("Date.prototype.getDay"),
        ("Date", "getUTCDay") => Some("Date.prototype.getUTCDay"),
        ("Date", "getHours") => Some("Date.prototype.getHours"),
        ("Date", "getUTCHours") => Some("Date.prototype.getUTCHours"),
        ("Date", "getMinutes") => Some("Date.prototype.getMinutes"),
        ("Date", "getUTCMinutes") => Some("Date.prototype.getUTCMinutes"),
        ("Date", "getSeconds") => Some("Date.prototype.getSeconds"),
        ("Date", "getUTCSeconds") => Some("Date.prototype.getUTCSeconds"),
        ("Date", "getMilliseconds") => Some("Date.prototype.getMilliseconds"),
        ("Date", "getUTCMilliseconds") => Some("Date.prototype.getUTCMilliseconds"),
        ("Date", "setTime") => Some("Date.prototype.setTime"),
        ("Date", "setMilliseconds") => Some("Date.prototype.setMilliseconds"),
        ("Date", "setUTCMilliseconds") => Some("Date.prototype.setUTCMilliseconds"),
        ("Date", "setSeconds") => Some("Date.prototype.setSeconds"),
        ("Date", "setUTCSeconds") => Some("Date.prototype.setUTCSeconds"),
        ("Date", "setMinutes") => Some("Date.prototype.setMinutes"),
        ("Date", "setUTCMinutes") => Some("Date.prototype.setUTCMinutes"),
        ("Date", "setHours") => Some("Date.prototype.setHours"),
        ("Date", "setUTCHours") => Some("Date.prototype.setUTCHours"),
        ("Date", "setDate") => Some("Date.prototype.setDate"),
        ("Date", "setUTCDate") => Some("Date.prototype.setUTCDate"),
        ("Date", "setMonth") => Some("Date.prototype.setMonth"),
        ("Date", "setUTCMonth") => Some("Date.prototype.setUTCMonth"),
        ("Date", "setFullYear") => Some("Date.prototype.setFullYear"),
        ("Date", "setUTCFullYear") => Some("Date.prototype.setUTCFullYear"),
        ("Date", "toLocaleString") => Some("Date.prototype.toLocaleString"),
        ("Date", "toUTCString") => Some("Date.prototype.toUTCString"),
        ("Promise", "then") => Some("Promise.prototype.then"),
        ("Promise", "catch") => Some("Promise.prototype.catch"),
        ("Promise", "finally") => Some("Promise.prototype.finally"),
        ("String", "toString") => Some("String.prototype.toString"),
        ("String", "valueOf") => Some("String.prototype.valueOf"),
        ("String", "charAt") => Some("String.prototype.charAt"),
        ("String", "charCodeAt") => Some("String.prototype.charCodeAt"),
        ("String", "indexOf") => Some("String.prototype.indexOf"),
        ("String", "lastIndexOf") => Some("String.prototype.lastIndexOf"),
        ("String", "split") => Some("String.prototype.split"),
        ("String", "substring") => Some("String.prototype.substring"),
        ("String", "toLowerCase") => Some("String.prototype.toLowerCase"),
        ("String", "toUpperCase") => Some("String.prototype.toUpperCase"),
        _ => None,
    }
}

pub(in crate::backend::direct_wasm) fn builtin_constructor_prototype_kind(
    name: &str,
) -> Option<StaticValueKind> {
    if name == "Function" {
        return Some(StaticValueKind::Function);
    }

    if matches!(
        name,
        "Object"
            | "Array"
            | "ArrayBuffer"
            | "SharedArrayBuffer"
            | "DataView"
            | "Date"
            | "RegExp"
            | "Map"
            | "Set"
            | "Error"
            | "EvalError"
            | "RangeError"
            | "ReferenceError"
            | "SyntaxError"
            | "TypeError"
            | "URIError"
            | "AggregateError"
            | "SuppressedError"
            | "Promise"
            | "WeakMap"
            | "WeakRef"
            | "WeakSet"
            | "Uint8Array"
            | "Int8Array"
            | "Uint16Array"
            | "Int16Array"
            | "Uint32Array"
            | "Int32Array"
            | "Float32Array"
            | "Float64Array"
            | "Uint8ClampedArray"
            | "BigInt64Array"
            | "BigUint64Array"
            | "Number"
            | "String"
            | "Boolean"
            | "BigInt"
            | "Symbol"
    ) {
        return Some(StaticValueKind::Object);
    }

    None
}

pub(in crate::backend::direct_wasm) fn builtin_prototype_number_value(
    object_name: &str,
    property_name: &str,
) -> Option<f64> {
    match (object_name, property_name) {
        ("Function", "length") | ("Array", "length") | ("String", "length") => Some(0.0),
        _ => None,
    }
}

pub(in crate::backend::direct_wasm) fn bound_function_prototype_call_builtin_name(
    target_name: &str,
) -> String {
    format!("{BOUND_FUNCTION_PROTOTYPE_CALL_PREFIX}{target_name}")
}

pub(in crate::backend::direct_wasm) fn parse_bound_function_prototype_call_builtin_name(
    name: &str,
) -> Option<&str> {
    name.strip_prefix(BOUND_FUNCTION_PROTOTYPE_CALL_PREFIX)
}

pub(in crate::backend::direct_wasm) fn builtin_member_function_name(
    object_name: &str,
    property_name: &str,
) -> Option<&'static str> {
    match (object_name, property_name) {
        ("Array", "isArray") => Some("Array.isArray"),
        ("JSON", "stringify") => Some("JSON.stringify"),
        ("Object", "create") => Some("Object.create"),
        ("Object", "getOwnPropertyDescriptor") => Some("Object.getOwnPropertyDescriptor"),
        ("Object", "getOwnPropertyNames") => Some("Object.getOwnPropertyNames"),
        ("Object", "getOwnPropertySymbols") => Some("Object.getOwnPropertySymbols"),
        ("Object", "getPrototypeOf") => Some("Object.getPrototypeOf"),
        ("Object", "defineProperty") => Some("Object.defineProperty"),
        ("Object", "defineProperties") => Some("Object.defineProperties"),
        ("Object", "freeze") => Some("Object.freeze"),
        ("Object", "isFrozen") => Some("Object.isFrozen"),
        ("Object", "is") => Some("Object.is"),
        ("Object", "isExtensible") => Some("Object.isExtensible"),
        ("Object", "isSealed") => Some("Object.isSealed"),
        ("Object", "keys") => Some("Object.keys"),
        ("Object", "preventExtensions") => Some("Object.preventExtensions"),
        ("Object", "seal") => Some("Object.seal"),
        ("Object", "setPrototypeOf") => Some("Object.setPrototypeOf"),
        ("Proxy", "revocable") => Some("Proxy.revocable"),
        ("Reflect", "apply") => Some("Reflect.apply"),
        ("Reflect", "construct") => Some("Reflect.construct"),
        ("Reflect", "deleteProperty") => Some("Reflect.deleteProperty"),
        ("Reflect", "defineProperty") => Some("Reflect.defineProperty"),
        ("Reflect", "get") => Some("Reflect.get"),
        ("Reflect", "getOwnPropertyDescriptor") => Some("Reflect.getOwnPropertyDescriptor"),
        ("Reflect", "getPrototypeOf") => Some("Reflect.getPrototypeOf"),
        ("Reflect", "has") => Some("Reflect.has"),
        ("Reflect", "isExtensible") => Some("Reflect.isExtensible"),
        ("Reflect", "ownKeys") => Some("Reflect.ownKeys"),
        ("Reflect", "preventExtensions") => Some("Reflect.preventExtensions"),
        ("Reflect", "set") => Some("Reflect.set"),
        ("Reflect", "setPrototypeOf") => Some("Reflect.setPrototypeOf"),
        ("Date", "now") => Some("Date.now"),
        ("Date", "parse") => Some("Date.parse"),
        ("Date", "UTC") => Some("Date.UTC"),
        ("Promise", "resolve") => Some("Promise.resolve"),
        ("Promise", "reject") => Some("Promise.reject"),
        ("Promise", "withResolvers") => Some("Promise.withResolvers"),
        ("String", "fromCharCode") => Some("String.fromCharCode"),
        ("Math", "abs") => Some("Math.abs"),
        ("Math", "acos") => Some("Math.acos"),
        ("Math", "asin") => Some("Math.asin"),
        ("Math", "atan") => Some("Math.atan"),
        ("Math", "atan2") => Some("Math.atan2"),
        ("Math", "ceil") => Some("Math.ceil"),
        ("Math", "cos") => Some("Math.cos"),
        ("Math", "exp") => Some("Math.exp"),
        ("Math", "floor") => Some("Math.floor"),
        ("Math", "log") => Some("Math.log"),
        ("Math", "max") => Some("Math.max"),
        ("Math", "min") => Some("Math.min"),
        ("Math", "pow") => Some("Math.pow"),
        ("Math", "random") => Some("Math.random"),
        ("Math", "round") => Some("Math.round"),
        ("Math", "sin") => Some("Math.sin"),
        ("Math", "sqrt") => Some("Math.sqrt"),
        ("Math", "tan") => Some("Math.tan"),
        _ => None,
    }
}

pub(in crate::backend::direct_wasm) fn builtin_member_number_value(
    object_name: &str,
    property_name: &str,
) -> Option<f64> {
    match (object_name, property_name) {
        ("Math", "E") => Some(std::f64::consts::E),
        ("Math", "LN2") => Some(std::f64::consts::LN_2),
        ("Math", "LN10") => Some(std::f64::consts::LN_10),
        ("Math", "LOG2E") => Some(std::f64::consts::LOG2_E),
        ("Math", "LOG10E") => Some(std::f64::consts::LOG10_E),
        ("Math", "PI") => Some(std::f64::consts::PI),
        ("Math", "SQRT1_2") => Some(std::f64::consts::FRAC_1_SQRT_2),
        ("Math", "SQRT2") => Some(std::f64::consts::SQRT_2),
        ("Number", "EPSILON") => Some(f64::EPSILON),
        ("Number", "MAX_SAFE_INTEGER") => Some(9_007_199_254_740_991.0),
        ("Number", "MAX_VALUE") => Some(f64::MAX),
        ("Number", "MIN_SAFE_INTEGER") => Some(-9_007_199_254_740_991.0),
        ("Number", "MIN_VALUE") => Some(f64::from_bits(1)),
        ("Number", "NaN") => Some(f64::NAN),
        ("Number", "NEGATIVE_INFINITY") => Some(f64::NEG_INFINITY),
        ("Number", "POSITIVE_INFINITY") => Some(f64::INFINITY),
        _ => None,
    }
}

pub(in crate::backend::direct_wasm) fn builtin_function_runtime_value(name: &str) -> Option<i32> {
    match name {
        "eval" => Some(JS_BUILTIN_EVAL_VALUE),
        TEST262_CREATE_REALM_BUILTIN => Some(JS_TYPEOF_FUNCTION_TAG),
        _ => builtin_runtime_function_offset(name)
            .map(|offset| JS_BUILTIN_FUNCTION_VALUE_BASE + offset as i32),
    }
    .or_else(|| parse_test262_realm_eval_builtin(name).map(|_| JS_TYPEOF_FUNCTION_TAG))
}

pub(in crate::backend::direct_wasm) fn builtin_function_runtime_entries()
-> impl Iterator<Item = (&'static str, i32)> {
    BUILTIN_RUNTIME_FUNCTION_NAMES
        .iter()
        .enumerate()
        .map(|(offset, name)| (*name, JS_BUILTIN_FUNCTION_VALUE_BASE + offset as i32))
}

fn builtin_runtime_function_offset(name: &str) -> Option<usize> {
    BUILTIN_RUNTIME_FUNCTION_NAMES
        .iter()
        .position(|candidate| *candidate == name)
}

pub(in crate::backend::direct_wasm) fn is_non_definable_global_name(name: &str) -> bool {
    matches!(name, "NaN" | "Infinity" | "undefined")
}
