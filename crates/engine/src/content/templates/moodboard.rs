//! Original vector imagery and editable color references; no remote photos or generated bitmaps.
use serde_json::json;

use super::kit::{BORDER, CARD, H, INK, Kit, MUTED, PRIMARY, SOFT, V, gradient};
use crate::Result;

pub(super) fn build(k: &mut Kit) -> Result<()> {
    k.foundations()?;
    k.rename_page("Moodboard")?;
    let page = k.page()?;
    let root = k.stack(page, "Moodboard DesignFlow", V, 24.0, json!(40), json!({"fill": CARD, "stroke": BORDER, "strokeWeight": 1, "sizingH": "fixed", "sizingV": "fixed", "width": 1200, "height": 800, "clip": true}))?;
    let heading =
        k.stack(root, "Direção criativa", H, 20.0, json!(0), json!({"sizingH": "fill", "mainAlign": "spaceBetween", "crossAlign": "center"}))?;
    k.text(heading, "Direção visual", "Heading/L", INK)?;
    k.text(heading, "MOODBOARD · DESIGNFLOW", "Label/M", PRIMARY)?;
    let collage = k.stack(root, "Referências vetoriais", H, 20.0, json!(0), json!({"sizingH": "fill"}))?;
    k.scene(collage, "Paisagem abstrata", [520.0, 380.0], json!({"cornerRadius": 20}))?;
    let plant = k.frame(
        collage,
        "Estudo botânico editável",
        [0.0, 0.0, 320.0, 380.0],
        json!({"fills": gradient("#F2EEFA", "#DED3EF"), "cornerRadius": 20, "clip": true}),
    )?;
    k.line(plant, "Caule", "M160 380 C145 270 179 190 155 60", "#6C5B7E", 5.0)?;
    for (index, (x, y, rotation)) in
        [(100.0, 70.0, -35.0), (165.0, 112.0, 32.0), (90.0, 172.0, -42.0), (165.0, 210.0, 35.0), (104.0, 272.0, -30.0), (170.0, 300.0, 34.0)]
            .iter()
            .enumerate()
    {
        k.ellipse(
            plant,
            &format!("Folha {}", index + 1),
            [*x, *y, 58.0, 106.0],
            json!({"fill": if index % 2 == 0 { "#8D78AB" } else { "#B49BD3" }, "rotation": rotation}),
        )?;
    }
    let texture = k.frame(collage, "Ritmo e textura vetorial", [0.0, 0.0, 240.0, 380.0], json!({"fill": SOFT, "cornerRadius": 20, "clip": true}))?;
    for index in 0..14 {
        let y = index as f64 * 36.0 - 60.0;
        k.line(
            texture,
            &format!("Faixa {}", index + 1),
            &format!("M-30 {y} C80 {} 170 {} 270 {}", y + 90.0, y - 10.0, y + 95.0),
            if index % 2 == 0 { "#C6AFF1" } else { "#D6C4F4" },
            16.0,
        )?;
    }
    let palette = k.stack(root, "Paleta de cores", H, 16.0, json!(0), json!({"sizingH": "fill"}))?;
    for color in ["#6B17F5", "#9461EE", "#B99BEF", "#D3BFFA", "#EEE7FF", "#FBFAFF"] {
        let swatch = k.stack(palette, color, V, 8.0, json!(0), json!({"sizingH": "fill"}))?;
        k.rect(
            swatch,
            "Amostra editável",
            [0.0, 0.0, 170.0, 82.0],
            json!({"fill": color, "cornerRadius": 12, "stroke": BORDER, "strokeWeight": 1, "sizingH": "fill"}),
        )?;
        k.text(swatch, color, "Label/S", MUTED)?;
    }
    k.para(root, "Use este painel para organizar sua paleta, composições e referências. Todas as formas e legendas são editáveis.", "Body/M", MUTED)?;
    Ok(())
}
