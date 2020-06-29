// Copyright (c) 2020 E.S.R.Labs. All rights reserved.
//
// NOTICE:  All information contained herein is, and remains
// the property of E.S.R.Labs and its suppliers, if any.
// The intellectual and technical concepts contained herein are
// proprietary to E.S.R.Labs and its suppliers and may be covered
// by German and Foreign Patents, patents in process, and are protected
// by trade secret or copyright law.
// Dissemination of this information or reproduction of this material
// is strictly forbidden unless prior written permission is obtained
// from E.S.R.Labs.

use super::Container;
use crate::{Name, State, SETTINGS};
use anyhow::{anyhow, Context, Result};
use async_std::{fs, path::Path};
use futures::stream::StreamExt;
use log::{debug, info};
use north_common::manifest::{Manifest, Version};
use std::{io::Read, process::Command, str::FromStr};

const MANIFEST: &str = "manifest.yaml";
const FS_IMAGE: &str = "fs.img";

pub async fn install_all(state: &mut State, dir: &Path) -> Result<()> {
    info!("Installing containers from {}", dir.display());

    lazy_static::lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(
            format!(
                r"^.*-{}-\d+\.\d+\.\d+\.npk$",
                env!("VERGEN_TARGET_TRIPLE")
            )
            .as_str(),
        )
        .expect("Invalid regex");
    }

    let containers = fs::read_dir(&dir)
        .await
        .with_context(|| format!("Failed to read {}", dir.display()))?
        .filter_map(move |d| async move { d.ok() })
        .map(|d| d.path())
        .filter_map(move |d| async move {
            if RE.is_match(&d.display().to_string()) {
                Some(d)
            } else {
                None
            }
        });

    let mut containers = Box::pin(containers);
    while let Some(container) = containers.next().await {
        install(state, &container).await?;
    }
    Ok(())
}

pub async fn install(state: &mut State, npk: &Path) -> Result<(Name, Version)> {
    let file =
        std::fs::File::open(&npk).with_context(|| format!("Failed to open {}", npk.display()))?;
    let reader = std::io::BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader).context("Failed to read zip")?;

    debug!("Loading manifest");
    let manifest = {
        let mut manifest_file = archive
            .by_name(MANIFEST)
            .with_context(|| format!("Failed to read manifest from {}", npk.display()))?;
        let mut manifest = String::new();
        manifest_file.read_to_string(&mut manifest)?;
        Manifest::from_str(&manifest)?
    };

    if state.applications.contains_key(&manifest.name) {
        return Err(anyhow!(
            "Cannot install container with name {} because it already exists",
            manifest.name
        ));
    }

    let instances = manifest.instances.unwrap_or(1);

    for instance in 0..instances {
        let mut manifest = manifest.clone();
        if instances > 1 {
            manifest.name.push_str(&format!("-{:03}", instance));
        }

        let root = SETTINGS.run_dir.join(&manifest.name);

        if root.exists().await {
            debug!("Removing {}", root.display());
            fs::remove_dir_all(&root)
                .await
                .with_context(|| format!("Failed to remove {}", root.display()))?;
        }

        let fs_offset = {
            let mut f = archive
                .by_name(FS_IMAGE)
                .with_context(|| format!("Failed to read manifest from {}", npk.display()))?;
            let mut fstype = [0u8; 4];
            f.read_exact(&mut fstype)?;
            if &fstype == b"hsqs" {
                debug!("Detected SquashFS file system");
            } else {
                unimplemented!("Only squashfs images are supported");
            }
            f.data_start()
        };

        debug!("Unsquashing {} to {}", npk.display(), root.display());
        let mut cmd = Command::new("unsquashfs");
        cmd.arg("-o");
        cmd.arg(fs_offset.to_string());
        cmd.arg("-f");
        cmd.arg("-d");
        cmd.arg(root.display().to_string());
        cmd.arg(npk.display().to_string());
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "Failed to unsquash: {:?}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let data = if SETTINGS.global_data_dir {
            SETTINGS.data_dir.clone()
        } else {
            SETTINGS.data_dir.join(&manifest.name)
        };

        if !data.exists().await {
            fs::create_dir_all(&data)
                .await
                .with_context(|| format!("Failed to create {}", data.display()))?;
        }

        // TODO: Should root/data be symlinked to SETTINGS.data_dir.manifest.name?

        info!("Installed {}:{}", manifest.name, manifest.version);

        let container = Container {
            root,
            data,
            manifest,
        };

        state.add(container)?;
    }

    Ok((manifest.name, manifest.version))
}

pub async fn uninstall(container: &Container) -> Result<()> {
    debug!("Removing {}", container.root.display());
    fs::remove_dir_all(&container.root)
        .await
        .with_context(|| format!("Failed to remove {}", container.root.display()))
}
