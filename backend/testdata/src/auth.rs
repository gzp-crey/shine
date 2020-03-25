use super::Config;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use std::error::Error;

#[derive(Debug, Serialize)]
pub struct RegistrationParams {
    name: String,
    password: String,
    email: Option<String>,
}

pub async fn populate_roles(cfg: &Config) -> Result<(), Box<dyn Error>> {
    let roles = vec![
        ("admin", vec!["moderator", "✔️"]),
        ("moderator", vec!["content_moderator", "content_moderator"]),
        ("content_moderator", vec![]),
        ("comment_moderator", vec![]),
        ("✔️", vec![]),
    ];

    let client = Client::new();

    for (role, _) in &roles {
        log::info!("registering role {}", role);

        let res = client
            .post(&format!("{}/api/roles/{}", cfg.auth, role))
            .header("x-sh-testing-token", &cfg.test_token)
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => {}
            StatusCode::CONFLICT => log::warn!("role {} already created", role),
            c => return Err(format!("Unexpected status code: {}", c).into()),
        }
    }

    for (role, inherit) in &roles {
        for i in inherit {
            log::info!("role {} inherits {}", role, i);

            let res = client
                .post(&format!("{}/auth/api/roles/{}/inherit/{}", cfg.auth, role, i))
                .header("x-sh-testing-token", &cfg.test_token)
                .send()
                .await?;

            match res.status() {
                StatusCode::OK => {}
                StatusCode::CONFLICT => log::warn!("role {} already inherited {}", role, i),
                c => return Err(format!("Unexpected status code: {}", c).into()),
            }
        }
    }

    Ok(())
}

pub async fn populate_users(cfg: &Config) -> Result<(), Box<dyn Error>> {
    let users = vec![
        ("gzp", "123", Some("gzp@example.com")),
        ("user1", "pass1", Some("user1@example.com")),
        ("user2", "pass2", None),
        ("I'm a ⛄", "with ugly pass as ⛄", None),
        ("/#?", "123", None),
    ];

    let client = Client::new();
    for (user, pass, email) in users {
        log::info!("registering user {}", user);

        let body = RegistrationParams {
            name: user.to_owned(),
            password: pass.to_owned(),
            email: email.map(|e| e.to_owned()),
        };

        let res = client
            .post(&format!("{}/api/users/register", cfg.auth))
            .header("x-sh-testing-token", &cfg.test_token)
            .json(&body)
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => {}
            StatusCode::CONFLICT => log::warn!("user {} already registered", user),
            c => return Err(format!("Unexpected status code: {}", c).into()),
        }
    }

    Ok(())
}
