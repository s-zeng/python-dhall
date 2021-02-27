use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyFloat, PyList, PyTuple};
use pyo3::{import_exception, wrap_pyfunction};
use serde_dhall::{NumKind, SimpleValue};
use thiserror::Error;

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, Serializer};

#[derive(Debug, Error)]
pub enum DhallPythonError {
    #[error("Conversion error: {error}")]
    InvalidConversion { error: serde_dhall::Error },
    #[error("Python Runtime exception: {error}")]
    PyErr { error: String },
    #[error("Dictionary key is not a string: {obj:?}")]
    DictKeyNotString { obj: PyObject },
    #[error("Invalid float: {x}")]
    InvalidFloat { x: String },
    #[error("Invalid type: {t}, Error: {e}")]
    InvalidCast { t: String, e: String },
    // NoneError doesn't have an impl for `Display`
    // See https://github.com/rust-lang-nursery/failure/issues/61
    // See https://github.com/rust-lang/rust/issues/42327#issuecomment-378324282
    // #[fail(display = "Error: {}", s)]
    // NoneError { s: String },
}

impl From<serde_dhall::Error> for DhallPythonError {
    fn from(error: serde_dhall::Error) -> DhallPythonError {
        DhallPythonError::InvalidConversion { error }
    }
}

impl From<DhallPythonError> for PyErr {
    fn from(h: DhallPythonError) -> PyErr {
        match h {
            DhallPythonError::InvalidConversion { error } => {
                PyErr::new::<PyTypeError, _>(format!("{}", error))
            }
            // TODO
            DhallPythonError::PyErr { error: _error } => PyErr::new::<PyTypeError, _>("PyErr"),
            DhallPythonError::InvalidCast { t: _t, e: _e } => {
                PyErr::new::<PyTypeError, _>("InvalidCast")
            }
            _ => PyErr::new::<PyTypeError, _>("Unknown reason"),
        }
    }
}

impl From<PyErr> for DhallPythonError {
    fn from(error: PyErr) -> DhallPythonError {
        // TODO: This should probably just have the underlying PyErr as an argument,
        // but this type is not `Sync`, so we just use the debug representation for now.
        DhallPythonError::PyErr {
            error: format!("{:?}", error),
        }
    }
}

import_exception!(json, JSONDecodeError);

#[pyfunction]
pub fn load(py: Python, fp: PyObject, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    // Temporary workaround for
    // https://github.com/PyO3/pyo3/issues/145
    let io: &PyAny = fp.extract(py)?;

    // Alternative workaround
    // fp.getattr(py, "seek")?;
    // fp.getattr(py, "read")?;

    // Reset file pointer to beginning See
    // https://github.com/PyO3/pyo3/issues/143 Note that we ignore the return
    // value, because `seek` does not strictly need to exist on the object
    let _success = io.call_method("seek", (0,), None);

    let s_obj = io.call_method0("read")?;
    loads(py, s_obj.to_object(py), kwargs)
}

// This function is a poor man's implementation of
// impl From<&str> for PyResult<PyObject>, which is not possible,
// because we have none of these types under our control.
// Note: Encoding param is deprecated and ignored.
#[pyfunction]
pub fn loads(py: Python, s: PyObject, kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    // This was moved out of the Python module code to enable benchmarking.
    loads_impl(py, s, kwargs)
}

#[pyfunction]
// ensure_ascii, check_circular, allow_nan, cls, indent, separators, default, sort_keys, kwargs = "**")]
#[allow(unused_variables)]
pub fn dumps(
    py: Python,
    obj: PyObject,
    sort_keys: Option<PyObject>,
    _kwargs: Option<&PyDict>,
) -> PyResult<PyObject> {
    let v = SerializePyObject {
        py,
        obj: obj.extract(py)?,
        sort_keys: match sort_keys {
            Some(sort_keys) => sort_keys.is_true(py)?,
            None => false,
        },
    };
    let s: Result<String, DhallPythonError> = serde_dhall::serialize(&v)
        .to_string()
        .map_err(|error| DhallPythonError::InvalidConversion { error });
    Ok(s?.to_object(py))
}

#[pyfunction]
pub fn dump(
    py: Python,
    obj: PyObject,
    fp: PyObject,
    _kwargs: Option<&PyDict>,
) -> PyResult<PyObject> {
    let s = dumps(py, obj, None, None)?;
    let fp_ref: &PyAny = fp.extract(py)?;
    fp_ref.call_method1("write", (s,))?;
    // TODO: Will this always return None?
    Ok(pyo3::Python::None(py))
}

/// A hyper-fast JSON encoder/decoder written in Rust
#[pymodule]
fn dhall(_py: Python, m: &PyModule) -> PyResult<()> {
    // See https://github.com/PyO3/pyo3/issues/171
    // Use JSONDecodeError from stdlib until issue is resolved.
    // py_exception!(_hyperjson, JSONDecodeError);
    // m.add("JSONDecodeError", py.get_type::<JSONDecodeError>());

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add_wrapped(wrap_pyfunction!(load))?;
    m.add_wrapped(wrap_pyfunction!(loads))?;
    m.add_wrapped(wrap_pyfunction!(dump))?;
    m.add_wrapped(wrap_pyfunction!(dumps))?;

    Ok(())
}

/// Convert from a `serde_json::Value` to a `pyo3::object:PyObject`.
pub fn from_json(py: Python, json: &SimpleValue) -> Result<PyObject, PyErr> {
    macro_rules! obj {
        ($x:ident) => {
            Ok($x.to_object(py))
        };
    }

    match json {
        SimpleValue::Num(x) => match x {
            NumKind::Bool(n) => obj!(n),
            NumKind::Natural(n) => obj!(n),
            NumKind::Integer(n) => obj!(n),
            NumKind::Double(n) => {
                let d = f64::from(n.clone());
                obj!(d)
            }
        },
        SimpleValue::Text(x) => obj!(x),
        SimpleValue::Optional(o) => match o {
            None => Ok(py.None()),
            Some(b) => {
                let n = b.as_ref();
                from_json(py, n)
            }
        },
        SimpleValue::List(vec) => {
            let solution = vec
                .iter()
                .map(|i| from_json(py, i).unwrap())
                .collect::<Vec<_>>();
            obj!(solution)
        }
        SimpleValue::Record(map) => {
            // TODO: put an omit_nones option here
            let solution: std::collections::HashMap<_, _> = map
                .iter()
                .map(|(key, value)| (key.clone(), from_json(py, value).unwrap()))
                .collect();
            obj!(solution)
        }
        SimpleValue::Union(name, val) => match val {
            None => obj!(name),
            Some(b) => {
                let n = b.as_ref();
                from_json(py, n)
            }
        },
    }
}

pub fn loads_impl(py: Python, s: PyObject, _kwargs: Option<&PyDict>) -> PyResult<PyObject> {
    let string_result: Result<String, _> = s.extract(py);
    match string_result {
        Ok(string) => {
            let json_val: std::result::Result<serde_dhall::SimpleValue, _> =
                serde_dhall::from_str(&string).parse();
            match json_val {
                Ok(val) => {
                    let py_obj = from_json(py, &val).expect("from_json");
                    return Ok(py_obj);
                }
                Err(e) => {
                    return Err(PyTypeError::new_err(format!("{:?}", e)));
                }
            }
        }
        Err(e) => {
            return Err(PyTypeError::new_err(format!(
                "the Dhall object must be str: {:?}",
                e
            )));
        }
    }
}

struct SerializePyObject<'p, 'a> {
    py: Python<'p>,
    obj: &'a PyAny,
    sort_keys: bool,
}

impl<'p, 'a> Serialize for SerializePyObject<'p, 'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        macro_rules! cast {
            ($f:expr) => {
                if let Ok(val) = PyTryFrom::try_from(self.obj) {
                    return $f(val);
                }
            };
        }

        macro_rules! extract {
            ($t:ty) => {
                if let Ok(val) = <$t as FromPyObject>::extract(self.obj) {
                    return val.serialize(serializer);
                }
            };
        }

        fn debug_py_err<E: ser::Error>(err: PyErr) -> E {
            E::custom(format_args!("{:?}", err))
        }

        cast!(|x: &PyDict| {
            let mut map = serializer.serialize_map(Some(x.len()))?;
            for (key, value) in x {
                if key.is_none() {
                    map.serialize_key("null")?;
                } else if let Ok(key) = key.extract::<bool>() {
                    map.serialize_key(if key { "true" } else { "false" })?;
                } else if let Ok(key) = key.str() {
                    let key = key.to_str().map_err(debug_py_err)?;
                    map.serialize_key(&key)?;
                } else {
                    return Err(ser::Error::custom(format_args!(
                        "Dictionary key is not a string: {:?}",
                        key
                    )));
                }
                map.serialize_value(&SerializePyObject {
                    py: self.py,
                    obj: value,
                    sort_keys: self.sort_keys,
                })?;
            }
            map.end()
        });

        cast!(|x: &PyList| {
            let mut seq = serializer.serialize_seq(Some(x.len()))?;
            for element in x {
                seq.serialize_element(&SerializePyObject {
                    py: self.py,
                    obj: element,
                    sort_keys: self.sort_keys,
                })?
            }
            seq.end()
        });
        cast!(|x: &PyTuple| {
            let mut seq = serializer.serialize_seq(Some(x.len()))?;
            for element in x {
                seq.serialize_element(&SerializePyObject {
                    py: self.py,
                    obj: element,
                    sort_keys: self.sort_keys,
                })?
            }
            seq.end()
        });

        extract!(String);
        extract!(bool);

        cast!(|x: &PyFloat| x.value().serialize(serializer));
        extract!(u64);
        extract!(i64);

        if self.obj.is_none() {
            return serializer.serialize_unit();
        }

        match self.obj.repr() {
            Ok(repr) => Err(ser::Error::custom(format_args!(
                "Value is not JSON serializable: {}",
                repr,
            ))),
            Err(_) => Err(ser::Error::custom(format_args!(
                "Type is not JSON serializable: {:?}",
                self.obj.get_type().name(),
            ))),
        }
    }
}

#[derive(Copy, Clone)]
struct HyperJsonValue<'a> {
    py: Python<'a>,
    parse_float: &'a Option<PyObject>,
    parse_int: &'a Option<PyObject>,
}

impl<'de, 'a> DeserializeSeed<'de> for HyperJsonValue<'a> {
    type Value = PyObject;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'a> HyperJsonValue<'a> {
    fn parse_primitive<E, T>(self, value: T, parser: &PyObject) -> Result<PyObject, E>
    where
        E: de::Error,
        T: ToString,
    {
        match parser.call1(self.py, (value.to_string(),)) {
            Ok(primitive) => Ok(primitive),
            Err(err) => Err(de::Error::custom(DhallPythonError::from(err))),
        }
    }
}

impl<'de, 'a> Visitor<'de> for HyperJsonValue<'a> {
    type Value = PyObject;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.to_object(self.py))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.parse_int {
            Some(parser) => self.parse_primitive(value, parser),
            None => Ok(value.to_object(self.py)),
        }
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.parse_int {
            Some(parser) => self.parse_primitive(value, parser),
            None => Ok(value.to_object(self.py)),
        }
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.parse_float {
            Some(parser) => self.parse_primitive(value, parser),
            None => Ok(value.to_object(self.py)),
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.to_object(self.py))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(self.py.None())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut elements = Vec::new();

        while let Some(elem) = seq.next_element_seed(self)? {
            elements.push(elem);
        }

        Ok(elements.to_object(self.py))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = BTreeMap::new();

        while let Some((key, value)) = map.next_entry_seed(PhantomData::<String>, self)? {
            entries.insert(key, value);
        }

        Ok(entries.to_object(self.py))
    }
}
