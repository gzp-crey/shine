use actix_files;
use actix_rt::SystemRunner;
use actix_web::web;
use serde::{Deserialize, Serialize};
use std::{
    cell::{Ref, RefCell},
    fmt,
    rc::Rc,
};
use tera::{Error as TeraError, Tera};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebConfig {
    pub tera_templates: String,
    pub web_folder: String,
}

#[derive(Debug)]
pub enum WebCreateError {
    ConfigureTera(TeraError),
}

impl fmt::Display for WebCreateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WebCreateError::ConfigureTera(err) => write!(f, "Error in tera configuration: {:?}", err),
        }
    }
}

struct Inner {
    tera: RefCell<Tera>,
}

#[derive(Clone)]
pub struct State(Rc<Inner>);

impl State {
    pub fn new(tera: Tera) -> Self {
        Self(Rc::new(Inner {
            tera: RefCell::new(tera),
        }))
    }

    pub fn tera(&self) -> Ref<Tera> {
        self.0.tera.borrow()
    }
}

#[derive(Clone)]
pub struct WebService {
    tera: Tera,
    web_folder: String,
    web_root: String,
}

impl WebService {
    pub fn create(_sys: &mut SystemRunner, config: &WebConfig, web_root: &str) -> Result<WebService, WebCreateError> {
        log::info!("Parsing tera templates");
        let tera = Tera::new(&config.tera_templates).map_err(|err| WebCreateError::ConfigureTera(err.into()))?;

        Ok(WebService {
            tera,
            web_folder: config.web_folder.clone(),
            web_root: web_root.to_owned(),
        })
    }

    pub fn configure(&self, services: &mut web::ServiceConfig) {
        let state = State::new(self.tera.clone());

        services.service(
            web::scope(&self.web_root)
                .data(state.clone())
                .service(actix_files::Files::new("/static", &self.web_folder)),
        );
    }
}
