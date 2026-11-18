//! Spreadsheet, CAD and JSON dumps of walls, so the window can write them
//! without going through MCP.

use std::fmt::Write as _;

use crate::elements::Wall;
use crate::geometry::Point2;
use crate::home::Home;
use crate::ids::WallId;
use crate::joins::wall_outlines;

fn n(v: f64) -> String {
    let text = format!("{v:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// Walls to write: the selected ones, or every wall of the current storey.
pub fn walls_to_export(home: &Home, selected: &[WallId]) -> Vec<Wall> {
    let picked: Vec<Wall> = home
        .walls
        .iter()
        .filter(|wall| selected.contains(&wall.id))
        .cloned()
        .collect();
    if picked.is_empty() {
        home.level_view(home.current_level()).walls
    } else {
        picked
    }
}

/// One row per wall. Units are centimeters; plan axes are x right, y down.
pub fn walls_csv(walls: &[Wall]) -> String {
    let mut out = String::from(
        "id,start_x,start_y,end_x,end_y,length_cm,thickness_cm,height_cm,height_end_cm,type,level,arc_extent,area_m2\n",
    );
    for wall in walls {
        let area = wall.length() * wall.height / 10_000.0;
        let h_end = wall.height_at_end.map(n).unwrap_or_default();
        let kind = wall.wall_type.as_deref().unwrap_or("");
        let level = wall.level.map(|id| id.to_string()).unwrap_or_default();
        let arc = wall
            .arc_extent
            .filter(|_| wall.is_arc())
            .map(n)
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            wall.id,
            n(wall.start.x),
            n(wall.start.y),
            n(wall.end.x),
            n(wall.end.y),
            n(wall.length()),
            n(wall.thickness),
            n(wall.height),
            h_end,
            kind,
            level,
            arc,
            n(area),
        );
    }
    out
}

/// The same walls as a JSON array, in the project's own compact shape.
pub fn walls_json(walls: &[Wall]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(walls)
}

/// Floor outlines and centerlines as DXF, Y flipped to CAD's Y-up.
pub fn walls_dxf(walls: &[Wall]) -> String {
    let outlines = wall_outlines(walls);
    let mut entities = String::new();
    for (wall, outline) in walls.iter().zip(outlines) {
        for i in 0..outline.len() {
            let a = outline[i];
            let b = outline[(i + 1) % outline.len()];
            line(&mut entities, "WALLS", a, b);
        }
        let axis = wall.centerline();
        for pair in axis.windows(2) {
            line(&mut entities, "AXIS", pair[0], pair[1]);
        }
        let mid = wall.point_at(0.5);
        let _ = write!(
            entities,
            "0\nTEXT\n8\nIDS\n10\n{}\n20\n{}\n40\n12\n1\n{}\n",
            n(mid.x),
            n(-mid.y),
            wall.id
        );
    }
    format!("0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n")
}

fn line(out: &mut String, layer: &str, a: Point2, b: Point2) {
    let _ = write!(
        out,
        "0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n11\n{}\n21\n{}\n",
        n(a.x),
        n(-a.y),
        n(b.x),
        n(-b.y)
    );
}

/// Bytes of `csv`, `dxf` or `json`.
pub fn walls_bytes(walls: &[Wall], ext: &str) -> Result<Vec<u8>, String> {
    match ext {
        "csv" => Ok(walls_csv(walls).into_bytes()),
        "dxf" => Ok(walls_dxf(walls).into_bytes()),
        "json" => walls_json(walls)
            .map(String::into_bytes)
            .map_err(|e| e.to_string()),
        _ => Err(format!("unknown wall export format {ext}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point2;
    use crate::ids::LevelId;

    fn wall(id: u64, x0: f64, y0: f64, x1: f64, y1: f64) -> Wall {
        Wall::new(WallId(id), Point2::new(x0, y0), Point2::new(x1, y1))
    }

    #[test]
    fn csv_dxf_and_json_name_the_wall_and_its_length() {
        let mut w = wall(1, 0.0, 0.0, 400.0, 0.0);
        w.wall_type = Some("drywall-95".into());
        w.level = Some(LevelId(2));
        let walls = [w];

        let csv = walls_csv(&walls);
        assert!(csv.starts_with("id,start_x"));
        assert!(csv.contains("w1,0,0,400,0,400,15,250,,drywall-95,lv2,,10"));

        let dxf = walls_dxf(&walls);
        assert!(dxf.starts_with("0\nSECTION") && dxf.ends_with("EOF\n"));
        assert!(dxf.contains("WALLS") && dxf.contains("AXIS") && dxf.contains("w1"));
        // Plan y-down becomes CAD y-up.
        assert!(dxf.contains("20\n-"));

        let json = walls_json(&walls).unwrap();
        assert!(json.contains("\"id\": \"w1\""));
        assert!(json.contains("drywall-95"));
    }

    #[test]
    fn export_picks_the_selection_or_the_current_storey() {
        let mut home = Home::new("casa");
        home.levels.push(crate::elements::Level {
            id: LevelId(1),
            name: "0".into(),
            elevation: 0.0,
            ..crate::elements::Level::default()
        });
        home.levels.push(crate::elements::Level {
            id: LevelId(2),
            name: "1".into(),
            elevation: 280.0,
            ..crate::elements::Level::default()
        });
        home.walls.push(wall(1, 0.0, 0.0, 400.0, 0.0));
        let mut upstairs = wall(2, 0.0, 0.0, 300.0, 0.0);
        upstairs.level = Some(LevelId(2));
        home.walls.push(upstairs);

        let all = walls_to_export(&home, &[]);
        assert_eq!(
            all.iter().map(|w| w.id).collect::<Vec<_>>(),
            vec![WallId(1)]
        );

        let picked = walls_to_export(&home, &[WallId(2)]);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].id, WallId(2));
    }

    #[test]
    fn walls_bytes_refuses_an_unknown_extension() {
        assert!(walls_bytes(&[], "pdf").is_err());
        assert!(!walls_bytes(&[], "csv").unwrap().is_empty());
    }
}
