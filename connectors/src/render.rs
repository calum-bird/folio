use anyhow::{Context, Result};
use minijinja::{context, Environment};
use serde::Serialize;

use crate::model::RenderedFile;

pub struct Renderer {
    env: Environment<'static>,
}

impl Renderer {
    pub fn new(templates: &[(&'static str, &'static str)]) -> Result<Self> {
        let mut env = Environment::new();
        for (name, source) in templates {
            env.add_template(name, source)
                .with_context(|| format!("register template {name}"))?;
        }

        Ok(Self { env })
    }

    pub fn render<T>(&self, template: &str, relative_path: String, entity: &T) -> Result<RenderedFile>
    where
        T: Serialize,
    {
        let loaded_template = self
            .env
            .get_template(template)
            .with_context(|| format!("load template {template}"))?;
        let contents = loaded_template
            .render(context! { entity => entity })
            .with_context(|| format!("render template {template}"))?;
        Ok(RenderedFile {
            relative_path,
            contents,
        })
    }
}

pub fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                return ch;
            }

            '_'
        })
        .collect()
}
