use std::{
    fs,
    path::{Path, PathBuf},
};

use egui::Color32;
use toml::{map::Map, Table, Value};

pub mod watcher;

pub fn get_config_file_path() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let config_file = Path::new("./.config/vonal/config.toml");
    home.join(config_file)
}

#[derive(Default)]
pub struct ConfigBuilder {
    config: Table,
}

impl ConfigBuilder {
    pub fn new() -> Result<Self, ConfigError> {
        let file_path = get_config_file_path();
        let file = fs::read_to_string(file_path).ok();
        let table = match file {
            Some(file) => file.parse::<Table>().map_err(|_| ConfigError::ParseError)?,
            None => Table::default(),
        };
        Ok(Self { config: table })
    }
    pub fn new_safe() -> Self {
        let table = Self::new().map(|s| s.config).unwrap_or_default();
        Self { config: table }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = get_config_file_path();
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;
        fs::write(path, self.config.to_string())?;
        Ok(())
    }

    pub fn get_or_create<T: FromConfig + ToConfig>(
        &mut self,
        name: &'static str,
        value: T,
    ) -> Result<T, ConfigError> {
        let entry = self.config.entry(name).or_insert_with(|| value.to_config());
        T::from_config(entry).ok_or(ConfigError::BadEntryError {
            name,
            message: None,
        })
    }

    pub fn group(
        &mut self,
        name: &'static str,
        build: impl FnOnce(&mut ConfigBuilder) -> Result<(), ConfigError>,
    ) -> Result<(), ConfigError> {
        let config = self
            .config
            .get(name)
            .and_then(|value| value.as_table())
            .cloned()
            .unwrap_or_default();

        let mut builder = Self { config };
        build(&mut builder)?;

        self.config
            .insert(name.to_string(), Value::Table(builder.config));

        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    ParseError,
    BadEntryError {
        #[allow(dead_code)]
        name: &'static str,
        message: Option<String>,
    },
}

pub trait FromConfig {
    fn from_config(raw: &Value) -> Option<Self>
    where
        Self: Sized;
}

pub trait ToConfig {
    fn to_config(self) -> Value;
}

impl FromConfig for Color32 {
    fn from_config(raw: &Value) -> Option<Self> {
        let value = raw.as_str()?;
        let mut split = value
            .trim_start_matches("rgb(")
            .trim_start_matches("rgba(")
            .trim_end_matches(')')
            .split(',');
        let r: u8 = split.next()?.trim().parse().ok()?;
        let g: u8 = split.next()?.trim().parse().ok()?;
        let b: u8 = split.next()?.trim().parse().ok()?;
        let a: u8 = split.next().unwrap_or("255").trim().parse().ok()?;
        Some(Color32::from_rgba_unmultiplied(r, g, b, a))
    }
}

impl ToConfig for Color32 {
    fn to_config(self) -> Value {
        let r = self.r();
        let g = self.g();
        let b = self.b();
        let a = self.a();
        if a == 255 {
            Value::String(format!("rgb({r}, {g}, {b})"))
        } else {
            Value::String(format!("rgba({r}, {g}, {b}, {a})"))
        }
    }
}

macro_rules! impl_for_integers {
    ($($int_type:ty),*) => {
        $(
            impl FromConfig for $int_type {
                fn from_config(raw: &Value) -> Option<Self> {
                    raw.as_integer().and_then(|x| Self::try_from(x).ok())
                }
            }
            impl ToConfig for $int_type {
                fn to_config(self) -> Value {
                    Value::Integer(self as i64)
                }
            }
        )*
    };
}

impl_for_integers!(i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);

macro_rules! impl_for_floats {
    ($($int_type:ty),*) => {
        $(
            impl FromConfig for $int_type {
                fn from_config(raw: &Value) -> Option<Self> {
                    Some(raw.as_float()? as $int_type)
                }
            }
            impl ToConfig for $int_type {
                fn to_config(self) -> Value {
                    Value::Float(self as f64)
                }
            }
        )*
    };
}

impl_for_floats!(f64, f32);

impl FromConfig for bool {
    fn from_config(raw: &Value) -> Option<Self> {
        raw.as_bool()
    }
}
impl ToConfig for bool {
    fn to_config(self) -> Value {
        Value::Boolean(self)
    }
}

impl FromConfig for String {
    fn from_config(raw: &Value) -> Option<Self> {
        Some(raw.as_str()?.to_string())
    }
}
impl ToConfig for String {
    fn to_config(self) -> Value {
        Value::String(self)
    }
}

impl<T: FromConfig> FromConfig for Vec<T> {
    fn from_config(raw: &Value) -> Option<Self> {
        raw.as_array()?
            .iter()
            .map(|item| T::from_config(item))
            .collect()
    }
}
impl<T: ToConfig> ToConfig for Vec<T> {
    fn to_config(self) -> Value {
        Value::Array(self.into_iter().map(|x| x.to_config()).collect())
    }
}

impl FromConfig for Map<String, Value> {
    fn from_config(raw: &Value) -> Option<Self> {
        raw.as_table().cloned()
    }
}
impl ToConfig for Map<String, Value> {
    fn to_config(self) -> Value {
        Value::Table(self)
    }
}

impl FromConfig for Value {
    fn from_config(raw: &Value) -> Option<Self> {
        Some(raw.clone())
    }
}
impl ToConfig for Value {
    fn to_config(self) -> Value {
        self
    }
}

#[derive(Debug)]
pub enum Dimension {
    Percentage(f64),
    Point(f64),
}

impl Dimension {
    pub fn get_points(&self, max: f64) -> f64 {
        match self {
            Dimension::Percentage(p) => p * max,
            Dimension::Point(p) => *p,
        }
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Self::Point(0.)
    }
}

impl FromConfig for Dimension {
    fn from_config(raw: &Value) -> Option<Self> {
        if let Some(value) = raw.as_str() {
            if value.contains('%') {
                let number: f64 = value.strip_suffix('%')?.parse().ok()?;
                return Some(Dimension::Percentage(number / 100.));
            }

            let number: f64 = value.parse().ok()?;
            Some(Dimension::Point(number))
        } else {
            Some(Dimension::Point(raw.as_float()?))
        }
    }
}

impl ToConfig for Dimension {
    fn to_config(self) -> Value {
        match self {
            Dimension::Percentage(p) => Value::String(format!("{}%", p * 100.)),
            Dimension::Point(p) => Value::Float(p),
        }
    }
}

impl<T: FromConfig> FromConfig for Option<T> {
    fn from_config(raw: &Value) -> Option<Self> {
        let is_none = raw.as_str().map(|value| value == "auto").unwrap_or(false);
        if is_none {
            Some(None)
        } else {
            Some(Some(T::from_config(raw)?))
        }
    }
}

impl<T: ToConfig> ToConfig for Option<T> {
    fn to_config(self) -> Value {
        match self {
            Some(t) => t.to_config(),
            None => Value::String("auto".into()),
        }
    }
}
