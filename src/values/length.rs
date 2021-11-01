use cssparser::*;
use crate::traits::{Parse, ToCss};
use crate::printer::Printer;
use std::fmt::Write;
use super::calc::Calc;
use super::percentage::Percentage;
use super::number::serialize_number;

/// https://drafts.csswg.org/css-values-4/#typedef-length-percentage
#[derive(Debug, Clone, PartialEq)]
pub enum LengthPercentage {
  Length(Length),
  Percentage(Percentage),
  Calc(Calc<LengthPercentage>)
}

impl Parse for LengthPercentage {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ()>> {
    match input.try_parse(Calc::parse) {
      Ok(Calc::Value(v)) => return Ok(*v),
      Ok(calc) => return Ok(LengthPercentage::Calc(calc)),
      _ => {}
    }

    if let Ok(length) = input.try_parse(|input| Length::parse(input)) {
      return Ok(LengthPercentage::Length(length))
    }

    if let Ok(percent) = input.try_parse(|input| Percentage::parse(input)) {
      return Ok(LengthPercentage::Percentage(percent))
    }

    Err(input.new_error_for_next_token())
  }
}

impl std::ops::Mul<f32> for LengthPercentage {
  type Output = Self;

  fn mul(self, other: f32) -> LengthPercentage {
    match self {
      LengthPercentage::Length(l) => LengthPercentage::Length(l * other),
      LengthPercentage::Percentage(p) => LengthPercentage::Percentage(Percentage(p.0 * other)),
      LengthPercentage::Calc(c) => LengthPercentage::Calc(c * other)
    }
  }
}

impl std::ops::Add<LengthPercentage> for LengthPercentage {
  type Output = Self;

  fn add(self, other: LengthPercentage) -> LengthPercentage {
    match self.add_recursive(&other) {
      Some(r) => r,
      None => self.add(other)
    }
  }
}

impl LengthPercentage {
  fn add_recursive(&self, other: &LengthPercentage) -> Option<LengthPercentage> {
    match (self, other) {
      (LengthPercentage::Length(a), LengthPercentage::Length(b)) => {
        if let Some(res) = a.add_recursive(b) {
          Some(LengthPercentage::Length(res))
        } else {
          None
        }
      },
      (LengthPercentage::Percentage(a), LengthPercentage::Percentage(b)) => Some(LengthPercentage::Percentage(Percentage(a.0 + b.0))),
      (LengthPercentage::Calc(Calc::Value(v)), other) => v.add_recursive(other),
      (other, LengthPercentage::Calc(Calc::Value(v))) => other.add_recursive(v),
      (LengthPercentage::Calc(Calc::Sum(a, b)), other) => {
        if let Some(res) = LengthPercentage::Calc(*a.clone()).add_recursive(other) {
          return Some(res.add(LengthPercentage::from(*b.clone())))
        }

        if let Some(res) = LengthPercentage::Calc(*b.clone()).add_recursive(other) {
          return Some(LengthPercentage::from(*a.clone()).add(res))
        }

        None
      }
      (other, LengthPercentage::Calc(Calc::Sum(a, b))) => {
        if let Some(res) = other.add_recursive(&LengthPercentage::Calc(*a.clone())) {
          return Some(res.add(LengthPercentage::from(*b.clone())))
        }

        if let Some(res) = other.add_recursive(&LengthPercentage::Calc(*b.clone())) {
          return Some(LengthPercentage::from(*a.clone()).add(res))
        }

        None
      }
      _ => None
    }
  }

  fn add(self, other: LengthPercentage) -> LengthPercentage {
    let mut a = self;
    let mut b = other;

    if a == 0.0 {
      return b
    }

    if b == 0.0 {
      return a
    }

    if a < 0.0 && b > 0.0 {
      std::mem::swap(&mut a, &mut b);
    }
    
    match (a, b) {
      (LengthPercentage::Calc(a), LengthPercentage::Calc(b)) => LengthPercentage::Calc(a + b),
      (LengthPercentage::Calc(Calc::Value(a)), b) => a.add(b),
      (a, LengthPercentage::Calc(Calc::Value(b))) => a.add(*b),
      (a, b) => LengthPercentage::Calc(Calc::Sum(Box::new(a.into()), Box::new(b.into())))
    }
  }
}

impl std::convert::Into<Calc<LengthPercentage>> for LengthPercentage {
  fn into(self) -> Calc<LengthPercentage> {
    match self {
      LengthPercentage::Calc(c) => c,
      b => Calc::Value(Box::new(b))
    }
  }
}

impl std::convert::From<Calc<LengthPercentage>> for LengthPercentage {
  fn from(calc: Calc<LengthPercentage>) -> LengthPercentage {
    LengthPercentage::Calc(calc)
  }
}

impl std::cmp::PartialEq<f32> for LengthPercentage {
  fn eq(&self, other: &f32) -> bool {
    match self {
      LengthPercentage::Length(a) => *a == *other,
      LengthPercentage::Percentage(a) => *a == *other,
      LengthPercentage::Calc(_) => false
    }
  }
}

impl std::cmp::PartialOrd<f32> for LengthPercentage {
  fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
    match self {
      LengthPercentage::Length(a) => a.partial_cmp(other),
      LengthPercentage::Percentage(a) => a.partial_cmp(other),
      LengthPercentage::Calc(_) => None
    }
  }
}

impl ToCss for LengthPercentage {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> std::fmt::Result where W: std::fmt::Write {
    match self {
      LengthPercentage::Length(length) => length.to_css(dest),
      LengthPercentage::Percentage(percent) => percent.to_css(dest),
      LengthPercentage::Calc(calc) => calc.to_css(dest)
    }
  }
}

/// `<length-percentage> | auto`
#[derive(Debug, Clone, PartialEq)]
pub enum LengthPercentageOrAuto {
  Auto,
  LengthPercentage(LengthPercentage)
}

impl Parse for LengthPercentageOrAuto {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ()>> {
    if input.try_parse(|i| i.expect_ident_matching("auto")).is_ok() {
      return Ok(LengthPercentageOrAuto::Auto);
    }

    if let Ok(percent) = input.try_parse(|input| LengthPercentage::parse(input)) {
      return Ok(LengthPercentageOrAuto::LengthPercentage(percent))
    }

    Err(input.new_error_for_next_token())
  }
}

impl ToCss for LengthPercentageOrAuto {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> std::fmt::Result where W: std::fmt::Write {
    use LengthPercentageOrAuto::*;
    match self {
      Auto => dest.write_str("auto"),
      LengthPercentage(l) => l.to_css(dest),
    }
  }
}

const PX_PER_IN: f32 = 96.0;
const PX_PER_CM: f32 = PX_PER_IN / 2.54;
const PX_PER_MM: f32 = PX_PER_CM / 10.0;
const PX_PER_Q: f32 = PX_PER_CM / 40.0;
const PX_PER_PT: f32 = PX_PER_IN / 72.0;
const PX_PER_PC: f32 = PX_PER_IN / 6.0;

/// https://www.w3.org/TR/css-values-3/#absolute-lengths
#[derive(Debug, Clone, PartialEq)]
pub enum AbsoluteLength {
  Px(f32),
  In(f32),
  Cm(f32),
  Mm(f32),
  Q(f32),
  Pt(f32),
  Pc(f32)
}

impl AbsoluteLength {
  pub fn to_px(&self) -> f32 {
    use AbsoluteLength::*;
    match self {
      Px(value) => *value,
      In(value) => value * PX_PER_IN,
      Cm(value) => value * PX_PER_CM,
      Mm(value) => value * PX_PER_MM,
      Q(value) => value * PX_PER_Q,
      Pt(value) => value * PX_PER_PT,
      Pc(value) => value * PX_PER_PC
    }
  }

  pub fn to_unit_value(&self) -> (f32, &str) {
    use AbsoluteLength::*;
    match self {
      Px(value) => (*value, "px"),
      In(value) => (*value, "in"),
      Cm(value) => (*value, "cm"),
      Mm(value) => (*value, "mm"),
      Q(value) => (*value, "q"),
      Pt(value) => (*value, "pt"),
      Pc(value) => (*value, "pc")
    }
  }
}

impl std::ops::Mul<f32> for AbsoluteLength {
  type Output = Self;

  fn mul(self, other: f32) -> AbsoluteLength {
    use AbsoluteLength::*;
    match self {
      Px(value) => Px(value * other),
      In(value) => In(value * other),
      Cm(value) => Cm(value * other),
      Mm(value) => Mm(value * other),
      Q(value) => Q(value * other),
      Pt(value) => Pt(value * other),
      Pc(value) => Pc(value * other),
    }
  }
}

impl std::ops::Add<AbsoluteLength> for AbsoluteLength {
  type Output = Self;

  fn add(self, other: AbsoluteLength) -> AbsoluteLength {
    use AbsoluteLength::*;
    match (self, other) {
      (Px(a), Px(b)) => Px(a + b),
      (In(a), In(b)) => In(a + b),
      (Cm(a), Cm(b)) => Cm(a + b),
      (Mm(a), Mm(b)) => Mm(a + b),
      (Q(a), Q(b)) => Q(a + b),
      (Pt(a), Pt(b)) => Pt(a + b),
      (Pc(a), Pc(b)) => Pc(a + b),
      (a, b) => Px(a.to_px() + b.to_px())
    }
  }
}

impl std::cmp::PartialEq<f32> for AbsoluteLength {
  fn eq(&self, other: &f32) -> bool {
    use AbsoluteLength::*;
    match self {
      Px(value) => value == other,
      In(value) => value == other,
      Cm(value) => value == other,
      Mm(value) => value == other,
      Q(value) => value == other,
      Pt(value) => value == other,
      Pc(value) => value == other,
    }
  }
}

impl std::cmp::PartialOrd<f32> for AbsoluteLength {
  fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
    use AbsoluteLength::*;
    match self {
      Px(value) => value.partial_cmp(other),
      In(value) => value.partial_cmp(other),
      Cm(value) => value.partial_cmp(other),
      Mm(value) => value.partial_cmp(other),
      Q(value) => value.partial_cmp(other),
      Pt(value) => value.partial_cmp(other),
      Pc(value) => value.partial_cmp(other),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelativeLength {
  Em(f32),
  Ex(f32),
  Ch(f32),
  Rem(f32),
  Vw(f32),
  Vh(f32),
  Vmin(f32),
  Vmax(f32)
}

impl RelativeLength {
  pub fn to_unit_value(&self) -> (f32, &str) {
    use RelativeLength::*;
    match self {
      Em(value) => (*value, "em"),
      Ex(value) => (*value, "ex"),
      Ch(value) => (*value, "ch"),
      Rem(value) => (*value, "rem"),
      Vw(value) => (*value, "vw"),
      Vh(value) => (*value, "vh"),
      Vmin(value) => (*value, "vmin"),
      Vmax(value) => (*value, "vmax")
    }
  }

  fn add_recursive(&self, other: &RelativeLength) -> Option<RelativeLength> {
    use RelativeLength::*;
    match (self, other) {
      (Em(a), Em(b)) => Some(Em(a + b)),
      (Ex(a), Ex(b)) => Some(Ex(a + b)),
      (Ch(a), Ch(b)) => Some(Ch(a + b)),
      (Rem(a), Rem(b)) => Some(Rem(a + b)),
      (Vw(a), Vw(b)) => Some(Vw(a + b)),
      (Vh(a), Vh(b)) => Some(Vh(a + b)),
      (Vmin(a), Vmin(b)) => Some(Vmin(a + b)),
      (Vmax(a), Vmax(b)) => Some(Vmax(a + b)),
      _ => None
    }
  }
}

impl std::ops::Mul<f32> for RelativeLength {
  type Output = Self;

  fn mul(self, other: f32) -> RelativeLength {
    use RelativeLength::*;
    match self {
      Em(value) => Em(value * other),
      Ex(value) => Ex(value * other),
      Ch(value) => Ch(value * other),
      Rem(value) => Rem(value * other),
      Vw(value) => Vw(value * other),
      Vh(value) => Vh(value * other),
      Vmin(value) => Vmin(value * other),
      Vmax(value) => Vmax(value * other),
    }
  }
}

impl std::cmp::PartialEq<f32> for RelativeLength {
  fn eq(&self, other: &f32) -> bool {
    use RelativeLength::*;
    match self {
      Em(value) => value == other,
      Ex(value) => value == other,
      Ch(value) => value == other,
      Rem(value) => value == other,
      Vw(value) => value == other,
      Vh(value) => value == other,
      Vmin(value) => value == other,
      Vmax(value) => value == other,
    }
  }
}

impl std::cmp::PartialOrd<f32> for RelativeLength {
  fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
    use RelativeLength::*;
    match self {
      Em(value) => value.partial_cmp(other),
      Ex(value) => value.partial_cmp(other),
      Ch(value) => value.partial_cmp(other),
      Rem(value) => value.partial_cmp(other),
      Vw(value) => value.partial_cmp(other),
      Vh(value) => value.partial_cmp(other),
      Vmin(value) => value.partial_cmp(other),
      Vmax(value) => value.partial_cmp(other),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Length {
  Absolute(AbsoluteLength),
  Relative(RelativeLength),
  Calc(Calc<Length>)
}

impl Parse for Length {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ()>> {
    match input.try_parse(Calc::parse) {
      Ok(Calc::Value(v)) => return Ok(*v),
      Ok(calc) => return Ok(Length::Calc(calc)),
      _ => {}
    }

    let location = input.current_source_location();
    let token = input.next()?;
    match *token {
      Token::Dimension { value, ref unit, .. } => {
        Ok(match_ignore_ascii_case! { unit,
          "px" => Length::Absolute(AbsoluteLength::Px(value)),
          "in" => Length::Absolute(AbsoluteLength::In(value)),
          "cm" => Length::Absolute(AbsoluteLength::Cm(value)),
          "mm" => Length::Absolute(AbsoluteLength::Mm(value)),
          "q" => Length::Absolute(AbsoluteLength::Q(value)),
          "pt" => Length::Absolute(AbsoluteLength::Pt(value)),
          "pc" => Length::Absolute(AbsoluteLength::Pc(value)),
          "em" => Length::Relative(RelativeLength::Em(value)),
          "ex" => Length::Relative(RelativeLength::Ex(value)),
          "ch" => Length::Relative(RelativeLength::Ch(value)),
          "rem" => Length::Relative(RelativeLength::Rem(value)),
          "vw" => Length::Relative(RelativeLength::Vw(value)),
          "vh" => Length::Relative(RelativeLength::Vh(value)),
          "vmin" => Length::Relative(RelativeLength::Vmin(value)),
          "vmax" => Length::Relative(RelativeLength::Vmax(value)),
          _ => return Err(location.new_unexpected_token_error(token.clone())),
        })
      },
      Token::Number { value, .. } => {
        // TODO: quirks mode only?
        Ok(Length::px(value))
      }
      ref token => return Err(location.new_unexpected_token_error(token.clone())),
    }
  }
}

impl ToCss for Length {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> std::fmt::Result where W: std::fmt::Write {
    use cssparser::ToCss;

    let (value, unit) = match self {
      Length::Absolute(a) => a.to_unit_value(),
      Length::Relative(r) => r.to_unit_value(),
      Length::Calc(c) => return c.to_css(dest)
    };

    if value == 0.0 {
      return dest.write_char('0')
    }

    let int_value = if value.fract() == 0.0 {
      Some(value as i32)
    } else {
      None
    };
    let token = Token::Dimension {
      has_sign: value < 0.0,
      value: value,
      int_value,
      unit: CowRcStr::from(unit)
    };
    if value.abs() < 1.0 {
      let mut s = String::new();
      token.to_css(&mut s)?;
      if value < 0.0 {
        dest.write_char('-')?;
        dest.write_str(s.trim_start_matches("-0"))
      } else {
        dest.write_str(s.trim_start_matches('0'))
      }
    } else {
      token.to_css(dest)
    }
  }
}

impl std::ops::Mul<f32> for Length {
  type Output = Self;

  fn mul(self, other: f32) -> Length {
    match self {
      Length::Absolute(a) => Length::Absolute(a * other),
      Length::Relative(a) => Length::Relative(a * other),
      Length::Calc(a) => Length::Calc(a * other)
    }
  }
}

impl std::ops::Add<Length> for Length {
  type Output = Self;

  fn add(self, other: Length) -> Length {
    match self.add_recursive(&other) {
      Some(r) => r,
      None => self.add(other)
    }
  }
}

impl Length {
  pub fn zero() -> Length {
    Length::Absolute(AbsoluteLength::Px(0.0))
  }

  pub fn px(px: f32) -> Length {
    Length::Absolute(AbsoluteLength::Px(px))
  }

  pub fn to_px(&self) -> Option<f32> {
    match self {
      Length::Absolute(a) => Some(a.to_px()),
      _ => None
    }
  }

  fn add_recursive(&self, other: &Length) -> Option<Length> {
    match (self, other) {
      (Length::Absolute(a), Length::Absolute(b)) => Some(Length::Absolute(a.clone() + b.clone())),
      (Length::Relative(a), Length::Relative(b)) => {
        if let Some(res) = a.add_recursive(b) {
          Some(Length::Relative(res))
        } else {
          None
        }
      },
      (Length::Calc(Calc::Value(v)), other) => v.add_recursive(other),
      (other, Length::Calc(Calc::Value(v))) => other.add_recursive(v),
      (Length::Calc(Calc::Sum(a, b)), other) => {
        if let Some(res) = Length::Calc(*a.clone()).add_recursive(other) {
          return Some(res.add(Length::from(*b.clone())))
        }

        if let Some(res) = Length::Calc(*b.clone()).add_recursive(other) {
          return Some(Length::from(*a.clone()).add(res))
        }

        None
      }
      (other, Length::Calc(Calc::Sum(a, b))) => {
        if let Some(res) = other.add_recursive(&Length::Calc(*a.clone())) {
          return Some(res.add(Length::from(*b.clone())))
        }

        if let Some(res) = other.add_recursive(&Length::Calc(*b.clone())) {
          return Some(Length::from(*a.clone()).add(res))
        }

        None
      }
      _ => None
    }
  }

  fn add(self, other: Length) -> Length {
    let mut a = self;
    let mut b = other;

    if a == 0.0 {
      return b
    }

    if b == 0.0 {
      return a
    }

    if a < 0.0 && b > 0.0 {
      std::mem::swap(&mut a, &mut b);
    }
    
    match (a, b) {
      (Length::Calc(a), Length::Calc(b)) => Length::Calc(a + b),
      (Length::Calc(Calc::Value(a)), b) => a.add(b),
      (a, Length::Calc(Calc::Value(b))) => a.add(*b),
      (a, b) => Length::Calc(Calc::Sum(Box::new(a.into()), Box::new(b.into())))
    }
  }
}

impl std::convert::Into<Calc<Length>> for Length {
  fn into(self) -> Calc<Length> {
    match self {
      Length::Calc(c) => c,
      b => Calc::Value(Box::new(b))
    }
  }
}

impl std::convert::From<Calc<Length>> for Length {
  fn from(calc: Calc<Length>) -> Length {
    Length::Calc(calc)
  }
}

impl std::cmp::PartialEq<f32> for Length {
  fn eq(&self, other: &f32) -> bool {
    match self {
      Length::Absolute(a) => *a == *other,
      Length::Relative(a) => *a == *other,
      Length::Calc(_) => false
    }
  }
}

impl std::cmp::PartialOrd<f32> for Length {
  fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
    match self {
      Length::Absolute(a) => a.partial_cmp(other),
      Length::Relative(a) => a.partial_cmp(other),
      Length::Calc(_) => None
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LengthOrNumber {
  Length(Length),
  Number(f32)
}

impl Parse for LengthOrNumber {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ()>> {
    // Parse number first so unitless numbers are not parsed as lengths.
    if let Ok(number) = input.try_parse(f32::parse) {
      return Ok(LengthOrNumber::Number(number))
    }

    if let Ok(length) = Length::parse(input) {
      return Ok(LengthOrNumber::Length(length))
    }

    Err(input.new_error_for_next_token())
  }
}

impl ToCss for LengthOrNumber {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> std::fmt::Result where W: std::fmt::Write {
    match self {
      LengthOrNumber::Length(length) => length.to_css(dest),
      LengthOrNumber::Number(number) => serialize_number(*number, dest)
    }
  }
}
