//! An editable, anonymous résumé. Every text is a prompt to replace, not a real person's data.
use serde_json::json;

use super::glyphs;
use super::kit::{BORDER, CARD, H, INK, Kit, MUTED, PRIMARY, SOFT, V};
use crate::Result;

pub(super) fn build(k: &mut Kit) -> Result<()> {
    k.foundations()?;
    k.rename_page("Currículo")?;
    let page = k.page()?;
    let root = k.stack(page, "Currículo · A4", H, 32.0, json!(48), json!({"fill": CARD, "stroke": BORDER, "strokeWeight": 1, "sizingH": "fixed", "sizingV": "fixed", "width": 794, "height": 1123, "clip": true}))?;
    let sidebar = k.stack(
        root,
        "Perfil e habilidades",
        V,
        24.0,
        json!(20),
        json!({"fill": SOFT, "cornerRadius": 16, "sizingH": "fixed", "sizingV": "fill", "width": 198}),
    )?;
    let avatar = k.stack(sidebar, "Avatar editável", V, 12.0, json!(12), json!({"sizingH": "fill", "crossAlign": "center"}))?;
    k.icon(avatar, &glyphs::USER, 72.0, PRIMARY)?;
    k.text(avatar, "SEU PERFIL", "Label/S", PRIMARY)?;
    let contact = k.stack(sidebar, "Contato", V, 12.0, json!(0), json!({"sizingH": "fill"}))?;
    k.text(contact, "CONTATO", "Label/M", PRIMARY)?;
    for label in ["Seu e-mail", "Seu telefone", "Seu portfólio", "Cidade e estado"] {
        k.para(contact, label, "Body/S", MUTED)?;
    }
    let skills = k.stack(sidebar, "Habilidades", V, 14.0, json!(0), json!({"sizingH": "fill"}))?;
    k.text(skills, "HABILIDADES", "Label/M", PRIMARY)?;
    for (index, label) in ["Design de interfaces", "Pesquisa", "Prototipação", "Comunicação", "Organização", "Colaboração"].iter().enumerate()
    {
        let row = k.stack(skills, label, V, 6.0, json!(0), json!({"sizingH": "fill"}))?;
        k.text(row, label, "Body/S", INK)?;
        let track = k.frame(row, "Nível editável", [0.0, 0.0, 150.0, 5.0], json!({"fill": "#D9CCF5", "cornerRadius": 3}))?;
        k.rect(track, "Preenchimento", [0.0, 0.0, 132.0 - index as f64 * 10.0, 5.0], json!({"fill": PRIMARY, "cornerRadius": 3}))?;
    }
    let body = k.stack(root, "Experiência e formação", V, 30.0, json!(0), json!({"sizingH": "fill"}))?;
    let heading = k.stack(body, "Identificação", V, 8.0, json!(0), json!({"sizingH": "fill"}))?;
    k.para(heading, "Seu nome", "Heading/L", INK)?;
    k.para(heading, "Sua área de atuação", "Body/L", PRIMARY)?;
    k.rect(heading, "Linha de destaque", [0.0, 0.0, 64.0, 4.0], json!({"fill": PRIMARY, "cornerRadius": 2}))?;
    let summary = k.stack(body, "Resumo profissional", V, 12.0, json!(0), json!({"sizingH": "fill"}))?;
    k.text(summary, "SOBRE MIM", "Label/M", PRIMARY)?;
    k.para(summary, "Apresente sua experiência, seus interesses e a contribuição que deseja levar ao próximo projeto.", "Body/M", MUTED)?;
    let experience = k.stack(body, "Experiência", V, 22.0, json!(0), json!({"sizingH": "fill"}))?;
    k.text(experience, "EXPERIÊNCIA", "Label/M", PRIMARY)?;
    for label in ["Experiência mais recente", "Experiência anterior"] {
        let item = k.stack(experience, label, V, 6.0, json!(0), json!({"sizingH": "fill"}))?;
        k.para(item, "Nome do cargo", "Heading/S", INK)?;
        k.para(item, "Nome da empresa · Período", "Body/S", PRIMARY)?;
        k.para(item, "Descreva suas responsabilidades e destaque uma entrega ou resultado relevante.", "Body/M", MUTED)?;
    }
    let education = k.stack(body, "Formação", V, 10.0, json!(0), json!({"sizingH": "fill"}))?;
    k.text(education, "FORMAÇÃO", "Label/M", PRIMARY)?;
    k.para(education, "Curso ou especialização", "Heading/S", INK)?;
    k.para(education, "Instituição de ensino · Período", "Body/S", MUTED)?;
    let languages = k.stack(body, "Idiomas", V, 8.0, json!(0), json!({"sizingH": "fill"}))?;
    k.text(languages, "IDIOMAS", "Label/M", PRIMARY)?;
    k.para(languages, "Idioma · Nível de proficiência", "Body/M", MUTED)?;
    Ok(())
}
