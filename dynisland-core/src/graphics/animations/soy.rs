//! # Soy
//! Rust interpolation library.
//! https://crates.io/crates/soy/0.2.0
//!
//! # Usage
//! The main trait used for interpolation is [`soy::Lerper`]. It requires a
//! single method, `calculate`, which calculates interpolation progression at a
//! given time.
//!
//! Example implementing linear interpolation, taken directly from `soy`'s
//! implementation:
//! ```
//! struct Linear;
//!
//! impl soy::Lerper for Linear {
//!     fn calculate(&self, t: f32) -> {
//!         t
//!     }
//! }
//! ```
//!
//! [0]: trait.Lerper.html
// #![deny(missing_docs)]

use core::{
    fmt,
    ops::{Add, Mul, Sub},
};
use std::{marker::PhantomData, str::FromStr};

use serde::{de::Visitor, ser::SerializeStruct};

/// Interpolate between two values given an interpolation method.
///
/// # Arguments:
/// - `lerper`: Interpolation method to use.
/// - `start`: Initial data point.
/// - `end`: Final data point.
/// - `t`: Amount to interpolate between the values.
///
/// # Usage
/// ```
///     let start = 5.0;
///     let end = 10.0;
///
///     let quarter = soy::lerp(soy::Linear, start, end, 0.25);
///     assert_eq!(quarter, 6.25);
///
///     let half_way = soy::lerp(soy::Linear, start, end, 0.5);
///     assert_eq!(half_way, 7.5);
/// ```
pub fn lerp<T, D>(lerper: T, start: D, end: D, t: f32) -> D
where
    T: Lerper,
    D: Copy,
    D: Add<Output = D>,
    D: Sub<Output = D>,
    D: Mul<f32, Output = D>,
{
    start + (end - start) * lerper.calculate(t)
}

/// Trait implemented by all interpolating methods.
pub trait Lerper {
    /// Given a timing function _y = f(t)_, this method calculates the _y_ value
    /// at the given _t_.
    fn calculate(&self, t: f32) -> f32;
}

/// Wrapper around [`Bezier::new`][0].
///
/// # Usage
/// ```
/// let ease = soy::cubic_bezier(0.17, 0.67, 0.83, 0.67);
/// let ease_in_out = soy::cubic_bezier(0.42, 0.0, 0.58, 1.0);
/// ```
///
/// [0]: struct.Bezier.html#method.new
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Bezier {
    Bezier::new(x1, y1, x2, y2)
}

#[derive(Debug, Clone, PartialEq, Copy)] //added Clone, PartialEq
/// Unit cubic bezier easing function.
pub struct Bezier {
    /// _x_ coordinate co-efficients.
    pub(crate) x: (f32, f32, f32),
    /// _y_ coordinate co-efficients.
    pub(crate) y: (f32, f32, f32),
}

impl FromStr for Bezier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(LINEAR),
            "ease" => Ok(EASE),
            "ease-in" => Ok(EASE_IN),
            "ease-out" => Ok(EASE_OUT),
            "ease-in-out" => Ok(EASE_IN_OUT),
            "on" => Ok(ON),
            "off" => Ok(OFF),
            _ => {
                //TODO maybe implement parser for point pair( "cubic-bezier(n,n,n,n)" )
                Err("Not a valid named curve".to_string())
            }
        }
    }
}

impl ToString for Bezier {
    fn to_string(&self) -> String {
        if *self == LINEAR {
            "linear".to_string()
        } else if *self == EASE {
            "ease".to_string()
        } else if *self == EASE_IN {
            "ease-in".to_string()
        } else if *self == EASE_OUT {
            "ease-out".to_string()
        } else if *self == EASE_IN_OUT {
            "ease-in-out".to_string()
        } else if *self == ON {
            "step-end".to_string()
        } else if *self == OFF {
            "step-start".to_string()
        } else {
            let (x1, y1, x2, y2) = self.control_points();
            format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2).to_string()
        }
    }
}

impl serde::Serialize for Bezier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if *self == LINEAR {
            serializer.serialize_str("linear")
        } else if *self == EASE {
            serializer.serialize_str("ease")
        } else if *self == EASE_IN {
            serializer.serialize_str("ease-in")
        } else if *self == EASE_OUT {
            serializer.serialize_str("ease-out")
        } else if *self == EASE_IN_OUT {
            serializer.serialize_str("ease-in-out")
        } else if *self == ON {
            serializer.serialize_str("on")
        } else if *self == OFF {
            serializer.serialize_str("off")
        } else {
            let mut state = serializer.serialize_struct("Bezier", 2)?;
            state.serialize_field("x", &self.x)?;
            state.serialize_field("y", &self.y)?;
            state.end()
        }
    }
}

impl<'de> serde::Deserialize<'de> for Bezier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            ///(x1, x2, x3)
            X,
            ///(y1, y2, y3)
            Y,
            ///(x1, y1)
            P1,
            ///(x2, y2)
            P2,
        }

        impl<'de> serde::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("(`x` and `y`) or (`p1` and `p2`)")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "x" => Ok(Field::X),
                            "y" => Ok(Field::Y),
                            "p1" => Ok(Field::P1),
                            "p2" => Ok(Field::P2),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        const FIELDS: &[&str] = &["x", "y", "p1", "p2"];

        struct BezierVisitor;

        impl<'de> serde::de::Visitor<'de> for BezierVisitor {
            type Value = Bezier;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("map")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Bezier, V::Error>
            where
                V: serde::de::SeqAccess<'de>,
            {
                let v1: f32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let v2: f32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                let v3: f32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
                let v4: f32 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;
                let v5: Option<f32> = seq.next_element()?.or(None);
                let v6: Option<f32> = seq.next_element()?.or(None);
                if let Some(v5) = v5 {
                    if v6.is_none() {
                        return Err(serde::de::Error::invalid_length(5, &self));
                    }
                    let v6 = v6.unwrap();
                    Ok(Bezier {
                        x: (v1, v2, v3),
                        y: (v4, v5, v6),
                    })
                } else {
                    Ok(cubic_bezier(v1, v2, v3, v4))
                }
            }

            fn visit_map<V>(self, mut map: V) -> Result<Bezier, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut x = None;
                let mut y = None;
                let mut p1 = None;
                let mut p2 = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::X => {
                            if x.is_some() {
                                return Err(serde::de::Error::duplicate_field("x"));
                            }
                            x = Some(map.next_value()?);
                        }
                        Field::Y => {
                            if y.is_some() {
                                return Err(serde::de::Error::duplicate_field("y"));
                            }
                            y = Some(map.next_value()?);
                        }
                        Field::P1 => {
                            if p1.is_some() {
                                return Err(serde::de::Error::duplicate_field("p1"));
                            }
                            p1 = Some(map.next_value()?);
                        }
                        Field::P2 => {
                            if p2.is_some() {
                                return Err(serde::de::Error::duplicate_field("p2"));
                            }
                            p2 = Some(map.next_value()?);
                        }
                    }
                }
                if x.is_some() && y.is_some() && p1.is_none() && p2.is_none() {
                    let x: (f32, f32, f32) = x.unwrap();
                    let y: (f32, f32, f32) = y.unwrap();
                    Ok(Bezier { x, y })
                } else if x.is_none() && y.is_none() && p1.is_some() && p2.is_some() {
                    let p1: (f32, f32) = p1.unwrap();
                    let p2: (f32, f32) = p2.unwrap();
                    Ok(Bezier::new(p1.0, p1.1, p2.0, p2.1))
                } else {
                    Err(serde::de::Error::custom(
                        "expecting (`x` and `y`) or (`p1` and `p2`)",
                    ))
                }
            }
        }
        deserializer.deserialize_struct("Bezier", FIELDS, BezierVisitor)
    }
}

impl Bezier {
    const NEWTON_ITERATIONS: usize = 8;
    // Assume duration of 1 second.
    const EPSILON: f32 = 1.0 / 200.0;

    /// Create a new cubic bezier, with provided _y_ values.
    ///
    /// # Usage
    /// ```
    /// let ease = soy::Bezier::new(0.25, 0.1, 0.25, 1.0);
    /// let ease_in_out = soy::Bezier::new(0.42, 0.0, 0.58, 1.0);
    /// ```
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Bezier {
        // Implementation based on WebKit's UnitBezier implementation.
        let cx = 3.0 * x1;
        let bx = 3.0 * (x2 - x1) - cx;
        let ax = 1.0 - cx - bx;

        let cy = 3.0 * y1;
        let by = 3.0 * (y2 - y1) - cy;
        let ay = 1.0 - cy - by;

        Bezier {
            x: (ax, bx, cx),
            y: (ay, by, cy),
        }
    }

    fn sample_x(&self, t: f32) -> f32 {
        let (a, b, c) = self.x;

        // Expanded "at^3 + bt^2 + ct"
        ((a * t + b) * t + c) * t
    }

    fn sample_y(&self, t: f32) -> f32 {
        let (a, b, c) = self.y;

        ((a * t + b) * t + c) * t
    }

    fn sample_derivative_x(&self, t: f32) -> f32 {
        let (a, b, c) = self.x;

        (3.0 * a * t + 2.0 * b) * t + c
    }

    fn solve_x(&self, x: f32) -> f32 {
        // Newton's method.
        let mut t = x;

        for _ in 0..Self::NEWTON_ITERATIONS {
            let x2 = self.sample_x(t);
            if approx_eq(x2, x, Self::EPSILON) {
                return t;
            }

            let dx = self.sample_derivative_x(t);
            if approx_eq(dx, 0.0, 1.0e-6) {
                break;
            }

            t -= (x2 - x) / dx;
        }

        // Fallback to bisection.
        let (mut low, mut high, mut t) = (0.0, 1.0, x);

        if t < low {
            return low;
        }
        if t > high {
            return high;
        }

        while low < high {
            let x2 = self.sample_x(t);
            if approx_eq(x2, x, Self::EPSILON) {
                return t;
            }
            if x > x2 {
                low = t;
            } else {
                high = t;
            }
            t = (high - low) / 2.0 + low;
        }

        // Fallback on failure.
        t
    }

    pub fn control_points(&self) -> (f32, f32, f32, f32) {
        let x1 = self.x.2 / 3.0;
        let x2 = (self.x.1 + (2.0 * self.x.2)) / 3.0;

        let y1 = self.y.2 / 3.0;
        let y2 = (self.y.1 + (2.0 * self.y.2)) / 3.0;

        (x1, y1, x2, y2)
    }

    pub fn from_string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: serde::Deserialize<'de> + FromStr<Err = String>,
        D: serde::Deserializer<'de>,
    {
        struct StringOrStructBezier<T>(PhantomData<fn() -> T>);

        impl<'de, T> Visitor<'de> for StringOrStructBezier<T>
        where
            T: serde::Deserialize<'de> + FromStr<Err = String>,
        {
            type Value = T;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    r#"map or (`linear` | `ease` | `ease-in` | `ease-out` | `ease-in-out`)"#,
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match <T as FromStr>::from_str(value) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(value),
                        &self,
                    )),
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                serde::Deserialize::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            }
            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                serde::Deserialize::deserialize(serde::de::value::SeqAccessDeserializer::new(seq))
            }
        }
        deserializer.deserialize_any(StringOrStructBezier(PhantomData))
    }
}
impl Lerper for Bezier {
    fn calculate(&self, t: f32) -> f32 {
        if *self == ON {
            return 1.0;
        } else if *self == OFF {
            return 0.0;
        }
        self.sample_y(self.solve_x(t))
    }
}

fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() < epsilon
}

/// Ease function, same as CSS's "ease" timing-function.
pub const EASE: Bezier = Bezier {
    x: (1.0, -0.75, 0.75),
    y: (-1.7, 2.4, 0.3),
};

/// Ease in function, same as CSS's "ease-in" timing-function.
pub const EASE_IN: Bezier = Bezier {
    x: (-0.74, 0.48, 1.26),
    y: (-2.0, 3.0, 0.0),
};

/// Ease out function, same as CSS's "ease-out" timing-function.
pub const EASE_OUT: Bezier = Bezier {
    x: (-0.74, 1.74, 0.0),
    y: (-2.0, 3.0, 0.0),
};

/// Ease in-out function, same as CSS's "ease-in-out" timing-function.
pub const EASE_IN_OUT: Bezier = Bezier {
    x: (0.52, -0.78, 1.26),
    y: (-2.0, 3.0, 0.0),
};

/// Linear function
pub const LINEAR: Bezier = Bezier {
    x: (0.0, 3.0, -2.0),
    y: (0.0, 3.0, -2.0),
};

pub const ON: Bezier = Bezier {
    x: (f32::INFINITY, f32::INFINITY, f32::INFINITY),
    y: (f32::INFINITY, f32::INFINITY, f32::INFINITY),
};

pub const OFF: Bezier = Bezier {
    x: (-f32::INFINITY, -f32::INFINITY, -f32::INFINITY),
    y: (-f32::INFINITY, -f32::INFINITY, -f32::INFINITY),
};
