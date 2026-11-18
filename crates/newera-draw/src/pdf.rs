//! Vector PDF of a plan scene, at a true architectural scale or fitted to the
//! page, with a small title block. Written by hand: PDF 1.4, the standard
//! Helvetica fonts (`WinAnsi` encoding) and uncompressed content.

use std::fmt::Write as _;

use newera_core::{Home, Point2, Wall};

use crate::scene::{Align, Color, Primitive, Scene, Size, wall_elevations_scene};

/// A4 portrait, centimeters.
const A4_CM: [f64; 2] = [21.0, 29.7];

/// Points per centimeter of paper.
const PT_PER_CM: f64 = 72.0 / 2.54;

#[derive(Debug, Clone, PartialEq)]
pub struct PdfOptions {
    /// Paper size in centimeters `[width, height]` (A3 landscape by default).
    pub paper_cm: [f64; 2],
    pub margin_cm: f64,
    /// Drawing scale denominator (100 for 1:100); `None` fits the page.
    pub scale: Option<f64>,
    pub title: String,
    /// Size in points of one output pixel for pixel-sized strokes.
    pub pt_per_px: f64,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            paper_cm: [42.0, 29.7],
            margin_cm: 1.0,
            scale: None,
            title: String::new(),
            pt_per_px: 0.45,
        }
    }
}

/// Maps characters to Windows-1252 bytes (Portuguese and common symbols);
/// others become `?`.
fn win_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c {
            '\u{20}'..='\u{7E}' | '\u{A0}'..='\u{FF}' => u8::try_from(u32::from(c)).unwrap_or(b'?'),
            '€' => 0x80,
            '‚' => 0x82,
            '„' => 0x84,
            '…' => 0x85,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '™' => 0x99,
            _ => b'?',
        })
        .collect()
}

fn escape(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 2);
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(char::from(b));
            }
            0x20..=0x7E => out.push(char::from(b)),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out
}

fn rgb(c: Color) -> (f64, f64, f64) {
    // Translucent colors are flattened onto white paper.
    let a = f64::from(c.0[3]) / 255.0;
    let f = |v: u8| (f64::from(v) / 255.0).mul_add(a, 1.0 - a);
    (f(c.0[0]), f(c.0[1]), f(c.0[2]))
}

/// Content stream of one page: the scene, fitted, plus the title block.
#[allow(clippy::too_many_lines)]
fn page_stream(scene: &Scene, options: &PdfOptions) -> String {
    let [paper_w, paper_h] = options.paper_cm.map(|v| v * PT_PER_CM);
    let margin = options.margin_cm * PT_PER_CM;
    let title_h = 1.2 * PT_PER_CM;
    let (min, max) = scene
        .bounds()
        .unwrap_or((Point2::new(0.0, 0.0), Point2::new(100.0, 100.0)));
    let (draw_w, draw_h) = ((max.x - min.x).max(1.0), (max.y - min.y).max(1.0));
    let (avail_w, avail_h) = (paper_w - 2.0 * margin, paper_h - 2.0 * margin - title_h);
    // Points per plan centimeter.
    let fit = (avail_w / draw_w).min(avail_h / draw_h);
    let scale = options
        .scale
        .map_or(fit, |denominator| PT_PER_CM / denominator.max(1.0));
    let (ox, oy) = (
        margin + (avail_w - draw_w * scale) / 2.0,
        margin + title_h + (avail_h - draw_h * scale) / 2.0,
    );
    let to = |p: Point2| (ox + (p.x - min.x) * scale, oy + (max.y - p.y) * scale);
    let size_pt = |s: Size| match s {
        Size::Px(px) => f64::from(px) * options.pt_per_px,
        Size::Cm(cm) => cm * scale,
    };

    let mut content = String::from("1 J 1 j\n");
    for item in &scene.items {
        match &item.primitive {
            Primitive::Fill { points, color, .. } if points.len() >= 3 => {
                let (r, g, b) = rgb(*color);
                let _ = writeln!(content, "{r:.3} {g:.3} {b:.3} rg");
                for (i, p) in points.iter().enumerate() {
                    let (x, y) = to(*p);
                    let _ = writeln!(content, "{x:.2} {y:.2} {}", if i == 0 { "m" } else { "l" });
                }
                content.push_str("h f\n");
            }
            Primitive::Line {
                points,
                closed,
                color,
                width,
            } if points.len() >= 2 => {
                let (r, g, b) = rgb(*color);
                let _ = writeln!(
                    content,
                    "{r:.3} {g:.3} {b:.3} RG {:.3} w",
                    size_pt(*width).max(0.1)
                );
                for (i, p) in points.iter().enumerate() {
                    let (x, y) = to(*p);
                    let _ = writeln!(content, "{x:.2} {y:.2} {}", if i == 0 { "m" } else { "l" });
                }
                content.push_str(if *closed { "h S\n" } else { "S\n" });
            }
            Primitive::Text {
                text,
                position,
                size,
                color,
                align,
                angle,
                look,
            } => {
                // Primitive sizes are glyph heights; Helvetica's is ~1.15 em.
                let em = size_pt(*size) / 1.15;
                if em < 0.8 {
                    continue;
                }
                let (r, g, b) = rgb(*color);
                let (x, y) = to(*position);
                let a = -angle.to_radians();
                let (sin, cos) = a.sin_cos();
                let lines: Vec<&str> = text.lines().collect();
                #[allow(clippy::cast_precision_loss)]
                let n = lines.len() as f64;
                let leading = em * 1.2;
                // Baseline of the first line relative to the anchor (y up).
                let first = match align {
                    Align::Center => (n - 1.0) * leading / 2.0 - em * 0.35,
                    Align::Above => (n - 1.0) * leading + em * 0.25,
                    Align::BaselineLeft | Align::BaselineCenter | Align::BaselineRight => {
                        (n - 1.0) * leading
                    }
                };
                let font = if look.bold { "/F2" } else { "/F1" };
                for (i, line) in lines.iter().enumerate() {
                    #[allow(clippy::cast_precision_loss)]
                    let width = line.chars().count() as f64 * em * 0.52;
                    let dx = -width * f64::from(align.horizontal());
                    #[allow(clippy::cast_precision_loss)]
                    let dy = first - i as f64 * leading;
                    let (tx, ty) = (x + dx * cos - dy * sin, y + dx * sin + dy * cos);
                    let _ = writeln!(
                        content,
                        "BT {font} {em:.2} Tf {r:.3} {g:.3} {b:.3} rg {cos:.4} {sin:.4} {:.4} {cos:.4} {tx:.2} {ty:.2} Tm ({}) Tj ET",
                        -sin,
                        escape(&win_ansi(line))
                    );
                }
            }
            _ => {}
        }
    }

    let scale_text = options.scale.map_or_else(
        || format!("Escala aprox. 1:{:.0}", PT_PER_CM / scale),
        |d| format!("Escala 1:{d:.0}"),
    );
    let _ = writeln!(
        content,
        "0.2 0.2 0.25 RG 0.6 w {margin:.2} {:.2} m {:.2} {:.2} l S\nBT /F2 10 Tf 0.1 0.1 0.12 rg {margin:.2} {:.2} Td ({}) Tj ET\nBT /F1 9 Tf 0.3 0.3 0.33 rg {:.2} {:.2} Td ({}) Tj ET",
        margin + title_h - 6.0,
        paper_w - margin,
        margin + title_h - 6.0,
        margin + 4.0,
        escape(&win_ansi(&options.title)),
        paper_w - margin - 150.0,
        margin + 4.0,
        escape(&win_ansi(&format!("{scale_text} · 3D New Era AI"))),
    );
    content
}

fn pdf_from_pages(streams: &[String], options: &PdfOptions) -> Vec<u8> {
    debug_assert!(!streams.is_empty());
    let [paper_w, paper_h] = options.paper_cm.map(|v| v * PT_PER_CM);
    let n = streams.len();
    let first_page = 3;
    let font1 = first_page + n * 2;
    let font2 = font1 + 1;
    let kids: String = (0..n)
        .map(|i| format!("{} 0 R", first_page + i * 2))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>"),
    ];
    for (i, content) in streams.iter().enumerate() {
        let page_id = first_page + i * 2;
        let content_id = page_id + 1;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {paper_w:.2} {paper_h:.2}] /Resources << /Font << /F1 {font1} 0 R /F2 {font2} 0 R >> >> /Contents {content_id} 0 R >>"
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ));
    }
    objects.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );
    objects.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );

    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(table.as_bytes());
    out
}

/// Renders the scene to PDF bytes.
pub fn to_pdf(scene: &Scene, options: &PdfOptions) -> Vec<u8> {
    pdf_from_pages(&[page_stream(scene, options)], options)
}

/// One wall per A4 portrait page, fitted, openings cut out.
pub fn walls_to_a4_pdf(home: &Home, walls: &[Wall], options: &PdfOptions) -> Vec<u8> {
    let mut page = options.clone();
    page.paper_cm = A4_CM;
    let streams: Vec<String> = if walls.is_empty() {
        vec![page_stream(&Scene::default(), &page)]
    } else {
        walls
            .iter()
            .map(|wall| {
                let mut one = page.clone();
                one.title = format!("{} / {}", options.title, wall.id);
                page_stream(
                    &wall_elevations_scene(home, std::slice::from_ref(wall)),
                    &one,
                )
            })
            .collect()
    };
    pdf_from_pages(&streams, &page)
}

/// A3 PDF of every wall as an elevation (length x height), openings cut out.
pub fn walls_to_pdf(home: &Home, walls: &[Wall], options: &PdfOptions) -> Vec<u8> {
    to_pdf(&wall_elevations_scene(home, walls), options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{SceneOptions, plan_scene};
    use newera_core::{Home, Label, Level, LevelId, Room, Wall};

    #[test]
    fn writes_a_structurally_valid_pdf_with_accents() {
        let mut home = Home::default();
        let pts = [(0.0, 0.0), (500.0, 0.0), (500.0, 400.0), (0.0, 400.0)];
        for i in 0..4 {
            let (a, b) = (pts[i], pts[(i + 1) % 4]);
            let id = home.new_wall_id();
            home.walls
                .push(Wall::new(id, Point2::new(a.0, a.1), Point2::new(b.0, b.1)));
        }
        let id = home.new_room_id();
        home.rooms.push(Room::new(
            id,
            "Cômodo — área",
            pts.iter().map(|&(x, y)| Point2::new(x, y)).collect(),
        ));
        let id = home.new_label_id();
        home.labels.push(Label {
            id,
            text: "Tomada (baixa)".into(),
            position: Point2::new(100.0, 100.0),
            ..Label::default()
        });
        let scene = plan_scene(&home, &SceneOptions::default());
        let pdf = to_pdf(
            &scene,
            &PdfOptions {
                scale: Some(50.0),
                title: "Casa".into(),
                ..PdfOptions::default()
            },
        );
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.trim_end().ends_with("%%EOF"));
        // "ô" (0xF4) and "—" (0x97) in WinAnsi, parentheses escaped.
        assert!(text.contains(r"C\364modo \227 \341rea"), "accents encoded");
        assert!(text.contains(r"Tomada \(baixa\)"));
        assert!(text.contains("Escala 1:50"));
        // The xref offset points at the table.
        let start = text.rfind("startxref\n").unwrap() + 10;
        let offset: usize = text[start..].lines().next().unwrap().parse().unwrap();
        assert!(pdf[offset..].starts_with(b"xref"));
    }

    #[test]
    fn walls_pdf_is_an_elevation_of_every_wall() {
        let mut home = Home::default();
        home.levels.push(Level {
            id: LevelId(1),
            name: "0".into(),
            elevation: 0.0,
            ..Level::default()
        });
        home.levels.push(Level {
            id: LevelId(2),
            name: "1".into(),
            elevation: 280.0,
            ..Level::default()
        });
        let ground = home.new_wall_id();
        home.walls.push(Wall::new(
            ground,
            Point2::new(0.0, 0.0),
            Point2::new(400.0, 0.0),
        ));
        let upstairs = home.new_wall_id();
        let mut upper = Wall::new(upstairs, Point2::new(0.0, 0.0), Point2::new(300.0, 0.0));
        upper.level = Some(LevelId(2));
        home.walls.push(upper);
        let label_id = home.new_label_id();
        home.labels.push(Label {
            id: label_id,
            text: "not a wall".into(),
            position: Point2::new(100.0, 100.0),
            ..Label::default()
        });
        let pdf = walls_to_pdf(
            &home,
            &home.walls,
            &PdfOptions {
                title: "Paredes".into(),
                ..PdfOptions::default()
            },
        );
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("Paredes"));
        assert!(text.contains(&format!("{ground}  400x250 cm")));
        assert!(text.contains(&format!("{upstairs}  300x250 cm")));
        assert!(!text.contains("not a wall"));
    }

    #[test]
    fn walls_a4_pdf_is_one_portrait_page_per_wall() {
        let mut home = Home::default();
        let a = home.new_wall_id();
        home.walls
            .push(Wall::new(a, Point2::new(0.0, 0.0), Point2::new(400.0, 0.0)));
        let b = home.new_wall_id();
        home.walls
            .push(Wall::new(b, Point2::new(0.0, 0.0), Point2::new(300.0, 0.0)));
        let pdf = walls_to_a4_pdf(
            &home,
            &home.walls,
            &PdfOptions {
                title: "Casa".into(),
                ..PdfOptions::default()
            },
        );
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/Count 2"));
        assert!(
            text.contains("595.28") && text.contains("841.89"),
            "{text:.80}"
        );
        assert!(text.contains(&format!("Casa / {a}")));
        assert!(text.contains(&format!("Casa / {b}")));
        assert!(text.contains(&format!("{a}  400x250 cm")));
        assert!(text.contains(&format!("{b}  300x250 cm")));
        let start = text.rfind("startxref\n").unwrap() + 10;
        let offset: usize = text[start..].lines().next().unwrap().parse().unwrap();
        assert!(pdf[offset..].starts_with(b"xref"));
    }
}
