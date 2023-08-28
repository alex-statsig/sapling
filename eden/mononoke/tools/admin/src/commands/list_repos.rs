/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use mononoke_app::MononokeApp;
use regex::Regex;

/// List known repositories
#[derive(Parser)]
pub struct CommandArgs {
    /// Pattern to match against repo names.
    pattern: Option<String>,
}

pub async fn run(app: MononokeApp, args: CommandArgs) -> Result<()> {
    let pattern = args
        .pattern
        .as_deref()
        .map(Regex::new)
        .transpose()
        .context("Failed to parse pattern")?;

    let configs = app.repo_configs();
    let mut repos = configs.repos.iter().collect::<Vec<_>>();
    repos.sort_unstable_by_key(|(_repo_name, repo_config)| repo_config.repoid);

    for (repo_name, repo_config) in repos.into_iter() {
        if let Some(pattern) = &pattern {
            if !pattern.is_match(repo_name) {
                continue;
            }
        }
        println!("{} {}", repo_config.repoid, repo_name);
    }

    Ok(())
}
