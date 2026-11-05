//! Bounded monochrome contour tracing. Raster data stays untouched.
use crate::params::{boolean, num, req_targets};
use crate::{Result, Session, bad};
use designflow_color::PaintKind;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt::Write, io::Cursor};

type Point = (u32, u32);

pub(super) fn vectorize(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "image.vectorize";
    let ids = req_targets(cmd, s, p)?;
    let [id] = ids[..] else {
        return Err(bad(cmd, "Selecione uma imagem por vez."));
    };
    let threshold = num(p, "threshold").unwrap_or(128.0);
    if !threshold.is_finite() || !(0.0..=255.0).contains(&threshold) {
        return Err(bad(cmd, "O limiar deve estar entre 0 e 255."));
    }
    let invert = boolean(p, "invert").unwrap_or(false);
    let doc = s.doc()?;
    let node = doc.get(id)?;
    let index = super::top_image_fill(node)
        .ok_or_else(|| bad(cmd, "A camada selecionada não contém imagem."))?;
    let PaintKind::Image(paint) = &node.fills[index].kind else {
        return Err(bad(cmd, "Preenchimento inválido."));
    };
    let data = doc
        .images
        .get(&paint.image)
        .ok_or_else(|| bad(cmd, "Dados da imagem indisponíveis."))?;
    let bounds = doc
        .world_bounds(id)
        .ok_or_else(|| bad(cmd, "Imagem sem limites."))?;
    let width = node.width;
    let name = format!("{} — Vetor P&B", node.name);
    let mut reader = image::ImageReader::new(Cursor::new(data.bytes.as_slice()))
        .with_guessed_format()
        .map_err(|e| bad(cmd, e.to_string()))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| bad(cmd, format!("Não foi possível ler a imagem: {e}")))?;
    let image = image.thumbnail(512, 512).to_rgba8();
    let (w, h) = image.dimensions();
    let mut mask = Vec::with_capacity((w * h) as usize);
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        let luma = (299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)) as f64 / 1000.0;
        mask.push(a >= 128 && ((luma < threshold) != invert));
    }
    let path = contours(&mask, w, h).map_err(|message| bad(cmd, message))?;
    if path.is_empty() {
        return Err(bad(
            cmd,
            "Nenhuma área encontrada. Ajuste o limiar ou inverta as cores.",
        ));
    }
    let height = width * f64::from(h) / f64::from(w);
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {w} {h}\"><path fill=\"black\" fill-rule=\"evenodd\" d=\"{path}\"/></svg>"
    );
    let page = s.page()?;
    let result = designflow_svg::import_svg_with_notes(
        s.doc_mut()?,
        svg.as_bytes(),
        page,
        bounds.x1 + 24.0,
        bounds.y0,
        &name,
    )
    .map_err(|e| bad(cmd, e.to_string()))?;
    s.set_selection(vec![result.frame]);
    Ok(json!({"id": result.frame}))
}

fn contours(mask: &[bool], width: u32, height: u32) -> std::result::Result<String, &'static str> {
    let filled = |x: i32, y: i32| {
        x >= 0
            && y >= 0
            && x < width as i32
            && y < height as i32
            && mask[(y as u32 * width + x as u32) as usize]
    };
    let mut edges: BTreeMap<Point, Vec<Point>> = BTreeMap::new();
    let mut count = 0;
    for y in 0..height {
        for x in 0..width {
            if !filled(x as i32, y as i32) {
                continue;
            }
            for (outside, start, end) in [
                (!filled(x as i32, y as i32 - 1), (x, y), (x + 1, y)),
                (!filled(x as i32 + 1, y as i32), (x + 1, y), (x + 1, y + 1)),
                (!filled(x as i32, y as i32 + 1), (x + 1, y + 1), (x, y + 1)),
                (!filled(x as i32 - 1, y as i32), (x, y + 1), (x, y)),
            ] {
                if outside {
                    edges.entry(start).or_default().push(end);
                    count += 1;
                    if count > 60000 {
                        return Err("Imagem muito detalhada. Use uma imagem com menos ruído.");
                    }
                }
            }
        }
    }
    let mut output = String::new();
    while let Some((&start, _)) = edges.first_key_value() {
        let mut current = start;
        let mut points = vec![start];
        loop {
            let outgoing = edges.get_mut(&current).ok_or("Contorno incompleto.")?;
            let next = outgoing.pop().ok_or("Contorno vazio.")?;
            if outgoing.is_empty() {
                edges.remove(&current);
            }
            if next == start {
                break;
            }
            points.push(next);
            current = next;
        }
        let mut first = true;
        for i in 0..points.len() {
            let previous = points[(i + points.len() - 1) % points.len()];
            let point = points[i];
            let next = points[(i + 1) % points.len()];
            let a = (
                point.0 as i64 - previous.0 as i64,
                point.1 as i64 - previous.1 as i64,
            );
            let b = (
                next.0 as i64 - point.0 as i64,
                next.1 as i64 - point.1 as i64,
            );
            if a.0 * b.1 == a.1 * b.0 {
                continue;
            }
            let _ = write!(
                output,
                "{}{} {}",
                if first { "M" } else { "L" },
                point.0,
                point.1
            );
            first = false;
        }
        output.push('Z');
    }
    Ok(output)
}

