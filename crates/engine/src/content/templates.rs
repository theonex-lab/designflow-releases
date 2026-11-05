//! Starter templates: documents to begin from, built in code by running commands (no template
//! files ship with the app). Everything in them is original and synthetic: the sample names,
//! people and figures are invented, the pictures are drawn from shapes and the icons are DesignFlow's
//! own ([`glyphs`]).
//!
//! Each template is real, editable content: shared colour and text styles, auto layout,
//! components with properties and instances of them, variables with modes, constraints and
//! prototype links. Building one is deterministic: the same document every time.

mod dashboard;
mod design_system;
mod glyphs;
mod icon_set;
mod kit;
mod landing;
mod mobile;
mod moodboard;
mod resume;
mod slides;
mod social;
mod wireframe;

use designflow_color::Color;
use designflow_doc::Document;
use serde_json::{Value, json};

use crate::cmd::CommandSpec;
use crate::params::string;
use crate::{DocState, Result, Session, bad, spec};

/// What a template is for (the home screen filters by it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    App,
    Website,
    Kit,
    Media,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::App, Kind::Website, Kind::Kit, Kind::Media];

    pub fn id(self) -> &'static str {
        match self {
            Kind::App => "app",
            Kind::Website => "website",
            Kind::Kit => "kit",
            Kind::Media => "media",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::App => "Apps",
            Kind::Website => "Websites",
            Kind::Kit => "Kits",
            Kind::Media => "Social and slides",
        }
    }
}

/// A starter template.
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    /// One line about what is in it.
    pub description: &'static str,
    pub kind: Kind,
    /// The size of its frames, as a tag ("390 × 844").
    pub size: &'static str,
    make: fn(&mut kit::Kit) -> Result<()>,
}

pub static TEMPLATES: &[Template] = &[
    Template {
        id: "mobile-app",
        name: "Aplicativo móvel",
        description: "Quatro telas editáveis com componentes e um fluxo de protótipo",
        kind: Kind::App,
        size: "390 × 844",
        make: mobile::build,
    },
    Template {
        id: "landing-page",
        name: "Landing page",
        description: "Uma página responsiva para apresentar sua ideia na web",
        kind: Kind::Website,
        size: "1440 de largura",
        make: landing::build,
    },
    Template {
        id: "dashboard",
        name: "Painel de dados",
        description: "Um painel editável com gráficos vetoriais, indicadores e uma tabela de exemplo",
        kind: Kind::App,
        size: "1440 × 960",
        make: dashboard::build,
    },
    Template {
        id: "design-system",
        name: "Identidade visual",
        description: "Cores, tipografia, variáveis de tema e componentes para sua marca",
        kind: Kind::Kit,
        size: "2 páginas",
        make: design_system::build,
    },
    Template {
        id: "wireframe-kit",
        name: "Kit de wireframes",
        description: "Componentes neutros e duas telas de exemplo para estruturar um fluxo",
        kind: Kind::Kit,
        size: "390 e 1280 de largura",
        make: wireframe::build,
    },
    Template {
        id: "social-posts",
        name: "Post para redes sociais",
        description: "Uma composição editável em cinco formatos para redes sociais",
        kind: Kind::Media,
        size: "5 formatos",
        make: social::build,
    },
    Template {
        id: "slides",
        name: "Apresentação",
        description: "Slides modernos com capa, agenda, destaque, gráfico e encerramento",
        kind: Kind::Media,
        size: "1920 × 1080",
        make: slides::build,
    },
    Template {
        id: "icon-set",
        name: "Kit de ícones",
        description: "Ícones vetoriais em uma grade de 24 px com guias e tamanhos",
        kind: Kind::Kit,
        size: "24 × 24",
        make: icon_set::build,
    },
    Template {
        id: "resume",
        name: "Currículo",
        description: "Um currículo profissional com perfil, experiência e formação editáveis",
        kind: Kind::Media,
        size: "794 × 1123",
        make: resume::build,
    },
    Template {
        id: "moodboard",
        name: "Moodboard",
        description: "Um painel de referências com arte vetorial e paleta de cores editável",
        kind: Kind::Kit,
        size: "1200 × 800",
        make: moodboard::build,
    },
];

pub fn find(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.id == id)
}

impl Template {
    /// A new document made from the template, its pages on `background`.
    pub fn build(&self, background: Color) -> Result<Document> {
        let mut kit = kit::Kit::new(self.name);
        (self.make)(&mut kit)?;
        kit.finish(background)
    }

    fn json(&self) -> Value {
        json!({"id": self.id, "name": self.name, "description": self.description, "kind": self.kind.id(), "size": self.size})
    }
}

fn list() -> Value {
    Value::Array(TEMPLATES.iter().map(Template::json).collect())
}

/// The longest document name `file.newFromTemplate` takes (chars).
const MAX_NAME: usize = 200;

fn new_from_template(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "file.newFromTemplate";
    let Some(id) = p.get("template") else {
        // Asked from a menu: the app shows the templates to pick from.
        s.request_ui("home.templates", json!({}));
        return Ok(json!({"templates": list()}));
    };
    let template = id.as_str().and_then(find).ok_or_else(|| bad(cmd, "`template` must be an id from `template.list`"))?;
    let mut doc = template.build(s.new_page_background())?;
    if let Some(name) = string(p, "name").map(|n| n.trim().chars().take(MAX_NAME).collect::<String>()).filter(|n| !n.is_empty()) {
        doc.name = name;
    }
    s.docs.push(DocState::new(doc, None));
    s.active = s.docs.len() - 1;
    Ok(json!({"index": s.active, "template": template.id}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "template.list",
            "List templates",
            "{}  the starter templates: [{id, name, description, kind (app|website|kit|media), size}]",
            false,
            |_, _| Ok(list())
        ),
        spec!(
            "file.newFromTemplate",
            "New from template",
            "{template (an id from `template.list`), name? (the document's name; default the template's)}  opens a new, unsaved document built from the template → {index, template}; without `template`, returns {templates} (and the app shows them to pick from)",
            false,
            new_from_template
        ),
    ]
}
