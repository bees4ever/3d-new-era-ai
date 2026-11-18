//! Electrosmog measurement zones: a polygon with height, drawn as its own
//! object so overlapping readings stay two (ADR 0007) and the number is
//! optional (ADR 0008).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::geometry::{Point2, polygon_area, polygon_centroid};
use crate::ids::{EsmogId, LevelId};
use crate::style::{Properties, is_zero};

fn invalid<T>(message: &str) -> CoreResult<T> {
    Err(CoreError::InvalidGeometry(message.to_owned()))
}

/// Units a baubiologist actually writes next to a reading.
pub const ESMOG_UNITS: &[&str] = &[
    "µW/m²", "mW/m²", "V/m", "nT", "µT", "mG", "A/m", "kHz", "MHz", "GHz",
];

/// A measured electrosmog region: plan polygon, extruded height, translucent
/// fill, and the reading drawn on it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EsmogZone {
    pub id: EsmogId,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub points: Vec<Point2>,
    /// Height of the measured volume, cm.
    pub height: f64,
    /// Bottom of the volume above the storey floor, cm.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub elevation: f64,
    /// The reading; missing means a region without a number yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Unit belonging to this reading.
    #[serde(
        default = "EsmogZone::default_unit",
        skip_serializing_if = "EsmogZone::is_default_unit"
    )]
    pub unit: String,
    #[serde(
        default = "EsmogZone::default_color",
        skip_serializing_if = "EsmogZone::is_default_color"
    )]
    pub color: [u8; 3],
    /// 0..1 fill opacity in plan and 3D.
    #[serde(
        default = "EsmogZone::default_opacity",
        skip_serializing_if = "EsmogZone::is_default_opacity"
    )]
    pub opacity: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<LevelId>,
    #[serde(default, skip_serializing_if = "Properties::is_empty")]
    pub properties: Properties,
}

impl EsmogZone {
    pub const DEFAULT_HEIGHT: f64 = 250.0;
    pub const DEFAULT_UNIT: &'static str = "µW/m²";
    pub const DEFAULT_COLOR: [u8; 3] = [220, 90, 40];
    pub const DEFAULT_OPACITY: f64 = 0.35;

    pub fn new(id: EsmogId, points: Vec<Point2>) -> Self {
        Self {
            id,
            name: String::new(),
            points,
            height: Self::DEFAULT_HEIGHT,
            elevation: 0.0,
            value: None,
            unit: Self::default_unit(),
            color: Self::DEFAULT_COLOR,
            opacity: Self::DEFAULT_OPACITY,
            level: None,
            properties: Properties::new(),
        }
    }

    fn default_unit() -> String {
        Self::DEFAULT_UNIT.to_owned()
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_default_unit(unit: &String) -> bool {
        unit == Self::DEFAULT_UNIT
    }

    const fn default_color() -> [u8; 3] {
        Self::DEFAULT_COLOR
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_default_color(color: &[u8; 3]) -> bool {
        *color == Self::DEFAULT_COLOR
    }

    const fn default_opacity() -> f64 {
        Self::DEFAULT_OPACITY
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_default_opacity(opacity: &f64) -> bool {
        (*opacity - Self::DEFAULT_OPACITY).abs() < f64::EPSILON
    }

    pub fn area(&self) -> f64 {
        polygon_area(&self.points)
    }

    pub fn centroid(&self) -> Option<Point2> {
        polygon_centroid(&self.points)
    }

    /// `800 µW/m²`, or empty when there is no number yet.
    pub fn reading_text(&self) -> String {
        match self.value {
            Some(v) if v.fract().abs() < 1e-9 => format!("{:.0} {}", v, self.unit),
            Some(v) => format!("{v:.2} {}", self.unit),
            None => String::new(),
        }
    }

    /// What the plan and 3D write on the zone.
    pub fn plan_label(&self) -> String {
        let reading = self.reading_text();
        match (self.name.trim().is_empty(), reading.is_empty()) {
            (false, false) => format!("{}\n{reading}", self.name.trim()),
            (false, true) => self.name.trim().to_owned(),
            (true, false) => reading,
            (true, true) => String::new(),
        }
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.points.len() < 3 {
            return invalid("esmog zone needs at least 3 points");
        }
        if self.points.iter().any(|p| !p.is_finite()) {
            return invalid("esmog zone points must be finite");
        }
        if self.area() < 1.0 {
            return invalid("esmog zone area must be positive");
        }
        if !(self.height.is_finite() && self.height > 0.0) {
            return invalid("esmog zone height must be positive");
        }
        if !self.elevation.is_finite() {
            return invalid("esmog zone elevation must be finite");
        }
        if let Some(v) = self.value
            && !v.is_finite()
        {
            return invalid("esmog reading must be finite");
        }
        if !(0.0..=1.0).contains(&self.opacity) {
            return invalid("esmog opacity must be between 0 and 1");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Home;
    use crate::ids::ElementId;

    fn square(id: u64, origin: f64) -> EsmogZone {
        EsmogZone::new(
            EsmogId(id),
            vec![
                Point2::new(origin, origin),
                Point2::new(origin + 200.0, origin),
                Point2::new(origin + 200.0, origin + 200.0),
                Point2::new(origin, origin + 200.0),
            ],
        )
    }

    #[test]
    fn overlapping_zones_stay_two_objects() {
        let mut home = Home::default();
        let mut a = square(1, 0.0);
        a.value = Some(800.0);
        let mut b = square(2, 50.0);
        b.value = Some(12.0);
        b.unit = "nT".into();
        home.insert(a.into(), None).unwrap();
        home.insert(b.into(), None).unwrap();
        assert_eq!(home.esmog.len(), 2);
        assert_eq!(home.esmog[0].value, Some(800.0));
        assert_eq!(home.esmog[1].unit, "nT");
        assert_eq!(
            home.element(ElementId::Esmog(EsmogId(1)))
                .unwrap()
                .id()
                .to_string(),
            "em1"
        );
    }

    #[test]
    fn a_zone_may_have_no_number() {
        let zone = square(3, 0.0);
        assert!(zone.value.is_none());
        assert!(zone.reading_text().is_empty());
        assert!(zone.plan_label().is_empty());
        zone.validate().unwrap();
    }

    #[test]
    fn reading_text_keeps_the_unit() {
        let mut zone = square(4, 0.0);
        zone.value = Some(0.5);
        zone.unit = "V/m".into();
        zone.name = "Schlafplatz".into();
        assert_eq!(zone.reading_text(), "0.50 V/m");
        assert_eq!(zone.plan_label(), "Schlafplatz\n0.50 V/m");
    }

    #[test]
    fn serde_keeps_two_overlapping_zones() {
        let mut home = Home::default();
        home.esmog.push(square(8, 0.0));
        home.esmog.push(square(9, 10.0));
        let json = serde_json::to_string(&home).unwrap();
        let back: Home = serde_json::from_str(&json).unwrap();
        assert_eq!(back.esmog.len(), 2);
        assert_eq!(back.esmog[0].id, EsmogId(8));
        assert_eq!(back.esmog[1].points[0].x, 10.0);
    }
}
