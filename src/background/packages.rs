use std::collections::HashMap;

use html5ever::tendril::TendrilSink;

use crate::{Package, PackageFile, PackageSrc, State, config};

#[tracing::instrument(skip(state))]
pub fn package_updates(state: std::sync::Arc<tokio::sync::RwLock<State>>) {
    let http_client = reqwest::blocking::Client::new();

    loop {
        tracing::trace!("Reloading package configuration");

        let config = match config::PackageConfiguration::load("./packages.toml") {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(?e, "Loading Package Configuration");
                std::thread::sleep(std::time::Duration::from_secs(15));
                continue;
            }
        };

        let mut new_packages: HashMap<String, _> = Default::default();

        for (pname, package_config) in config.package {
            tracing::trace!(?pname, "Handling package {:?}", package_config);

            if let Some(index_name) = &package_config.index {
                match load_package_index(
                    &http_client,
                    &config.index,
                    &pname,
                    &index_name,
                    &package_config,
                ) {
                    Ok(package) => {
                        new_packages.insert(pname, package);
                    }
                    Err(e) => {
                        tracing::error!(?e, "Loading Package from index");
                    }
                };

                continue;
            }
        }

        {
            let mut state = state.blocking_write();
            state.packages = new_packages;
        }

        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

#[derive(Debug)]
enum LoadPackageIndexError {
    UnknownIndex(String),
    InvalidIndexUrl,
    JoiningUrls,
    SendingRequest,
    ParseResponse(std::io::Error),
}

#[tracing::instrument(skip(http_client, index_config, package_config))]
fn load_package_index(
    http_client: &reqwest::blocking::Client,
    index_config: &HashMap<String, config::IndexConfigEntry>,
    pname: &str,
    index_name: &str,
    package_config: &config::PackageConfigEntry,
) -> Result<Package, LoadPackageIndexError> {
    tracing::trace!(?pname, "Handling package {:?}", package_config);

    let index = match index_config.get(index_name) {
        Some(i) => i,
        None => return Err(LoadPackageIndexError::UnknownIndex(index_name.to_string())),
    };

    tracing::trace!("Using Index {:?}", index);

    let base_url = reqwest::Url::parse(&index.url).map_err(|e| LoadPackageIndexError::InvalidIndexUrl)?;
    let target_url = base_url.join(&format!("{}/", pname)).map_err(|e| LoadPackageIndexError::JoiningUrls)?;
    tracing::trace!("Loading package files from '{}'", target_url);

    // TODO
    // Support authentication for the index

    let req_builder = http_client.get(target_url);

    let mut response = req_builder.send().map_err(|e| LoadPackageIndexError::SendingRequest)?;

    let parsing_opts = html5ever::ParseOpts {
        tree_builder: html5ever::tree_builder::TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let dom = html5ever::parse_document(markup5ever_rcdom::RcDom::default(), parsing_opts)
        .from_utf8()
        .read_from(&mut response)
        .map_err(|e| LoadPackageIndexError::ParseResponse(e))?;

    let mut files = Vec::new();

    let mut stack: Vec<markup5ever_rcdom::Handle> = dom.document.children.borrow().clone();
    while let Some(node) = stack.pop() {
        match &node.data {
            markup5ever_rcdom::NodeData::Element { name, attrs, .. }
                if "a" == name.local.as_ref() =>
            {
                let attrs = attrs.borrow();
                let link = attrs
                    .as_slice()
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "href");

                let children = node.children.borrow();
                let text = children
                    .as_slice()
                    .iter()
                    .find_map(|child| match &child.data {
                        markup5ever_rcdom::NodeData::Text { contents } => {
                            Some(contents.borrow().clone())
                        }
                        _ => None,
                    });

                let raw_url: &str = match link.map(|v| v.value.as_ref()) {
                    Some(v) => v,
                    None => {
                        tracing::warn!("");
                        continue;
                    }
                };

                let name: String = match text.map(|v| v.to_string()) {
                    Some(v) => v,
                    None => {
                        tracing::warn!("");
                        continue;
                    }
                };

                let url = match reqwest::Url::parse(raw_url) {
                    Ok(u) => u,
                    Err(e) => {
                        tracing::warn!(?e, "Parse URL in href");
                        continue;
                    }
                };

                files.push(PackageFile::RemotePackage {
                    name,
                    url,
                    auth: crate::RemotePackageAuth::Unauthorized, // TODO
                });
            }
            _ => {}
        };

        stack.extend_from_slice(node.children.borrow().as_slice());
    }

    Ok(Package {
        src: PackageSrc::Index { url: base_url },
        files,
    })
}
