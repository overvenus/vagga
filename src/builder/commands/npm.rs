use std::fs::File;
use std::path::{Path, PathBuf};

use quire::validate as V;
use unshare::{Stdio};
use rustc_serialize::json::Json;

use super::super::context::{Context};
use super::super::packages;
use builder::distrib::{Distribution, DistroBox};
use builder::commands::generic::{command, run};
use build_step::{BuildStep, VersionError, StepError, Digest, Config, Guard};


#[derive(RustcDecodable, Debug, Clone)]
pub struct NpmConfig {
    pub install_node: bool,
    pub npm_exe: String,
}

impl NpmConfig {
    pub fn config() -> V::Structure<'static> {
        V::Structure::new()
        .member("npm_exe", V::Scalar::new().default("npm"))
        .member("install_node", V::Scalar::new().default(true))
    }
}

#[derive(Debug)]
pub struct NpmInstall(Vec<String>);
tuple_struct_decode!(NpmInstall);

impl NpmInstall {
    pub fn config() -> V::Sequence<'static> {
        V::Sequence::new(V::Scalar::new())
    }
}

#[derive(RustcDecodable, Debug)]
pub struct NpmDependencies {
    pub file: PathBuf,
    pub package: bool,
    pub dev: bool,
    pub peer: bool,
    pub bundled: bool,
    pub optional: bool,
}

impl NpmDependencies {
    pub fn config() -> V::Structure<'static> {
        V::Structure::new()
        .member("file", V::Scalar::new().default("package.json"))
        .member("package", V::Scalar::new().default(true))
        .member("dev", V::Scalar::new().default(true))
        .member("peer", V::Scalar::new().default(false))
        .member("bundled", V::Scalar::new().default(true))
        .member("optional", V::Scalar::new().default(false))
    }
}

impl Default for NpmConfig {
    fn default() -> NpmConfig {
        NpmConfig {
            install_node: true,
            npm_exe: "npm".to_string(),
        }
    }
}

fn scan_features(settings: &NpmConfig, pkgs: &Vec<String>)
    -> Vec<packages::Package>
{
    let mut res = vec!();
    res.push(packages::BuildEssential);
    if settings.install_node {
        res.push(packages::NodeJs);
        res.push(packages::NodeJsDev);
        res.push(packages::Npm);
    }
    for name in pkgs.iter() {
        parse_feature(&name, &mut res);
    }
    return res;
}

pub fn parse_feature(info: &str, features: &mut Vec<packages::Package>) {
    // Note: the info is a package name/git-url in NpmInstall but it's just
    // a version number for NpmDependencies. That's how npm works.
    if info[..].starts_with("git://") {
        features.push(packages::Git);
    } // TODO(tailhook) implement whole a lot of other npm version kinds
}

pub fn ensure_npm(distro: &mut Box<Distribution>, ctx: &mut Context,
    features: &[packages::Package])
    -> Result<(), StepError>
{
    packages::ensure_packages(distro, ctx, features)
}

pub fn npm_install(distro: &mut Box<Distribution>, ctx: &mut Context,
    pkgs: &Vec<String>)
    -> Result<(), StepError>
{
    ctx.add_cache_dir(Path::new("/tmp/npm-cache"),
                           "npm-cache".to_string())?;
    let features = scan_features(&ctx.npm_settings, pkgs);
    ensure_npm(distro, ctx, &features)?;

    if pkgs.len() == 0 {
        return Ok(());
    }

    let mut cmd = command(ctx, &ctx.npm_settings.npm_exe)?;
    cmd.arg("install");
    cmd.arg("--global");
    cmd.arg("--user=root");
    cmd.arg("--cache=/tmp/npm-cache");
    cmd.args(pkgs);
    run(cmd)
}

fn scan_dic(json: &Json, key: &str,
    packages: &mut Vec<String>, features: &mut Vec<packages::Package>)
    -> Result<(), StepError>
{
    match json.find(key) {
        Some(&Json::Object(ref ob)) => {
            for (k, v) in ob {
                if !v.is_string() {
                    return Err(StepError::Compat(format!(
                        "Package {:?} has wrong version {:?}", k, v)));
                }
                let s = v.as_string().unwrap();
                parse_feature(&s, features);
                packages.push(format!("{}@{}", k, s));
                // TODO(tailhook) check the feature
            }
            Ok(())
        }
        None => {
            Ok(())
        }
        Some(_) => {
            Err(StepError::Compat(format!(
                "The {:?} is not a mapping (JSON object)", key)))
        }
    }
}

pub fn npm_deps(distro: &mut Box<Distribution>, ctx: &mut Context,
    info: &NpmDependencies)
    -> Result<(), StepError>
{
    ctx.add_cache_dir(Path::new("/tmp/npm-cache"),
                           "npm-cache".to_string())?;
    let mut features = scan_features(&ctx.npm_settings, &Vec::new());

    let json = File::open(&Path::new("/work").join(&info.file))
        .map_err(|e| format!("Error opening file {:?}: {}", info.file, e))
        .and_then(|mut f| Json::from_reader(&mut f)
        .map_err(|e| format!("Error parsing json {:?}: {}", info.file, e)))?;
    let mut packages = vec![];

    if info.package {
        scan_dic(&json, "dependencies", &mut packages, &mut features)?;
    }
    if info.dev {
        scan_dic(&json, "devDependencies", &mut packages, &mut features)?;
    }
    if info.peer {
        scan_dic(&json, "peerDependencies",
            &mut packages, &mut features)?;
    }
    if info.bundled {
        scan_dic(&json, "bundledDependencies",
            &mut packages, &mut features)?;
        scan_dic(&json, "bundleDependencies",
            &mut packages, &mut features)?;
    }
    if info.optional {
        scan_dic(&json, "optionalDependencies",
            &mut packages, &mut features)?;
    }

    ensure_npm(distro, ctx, &features)?;

    if packages.len() == 0 {
        return Ok(());
    }

    let mut cmd = command(ctx, &ctx.npm_settings.npm_exe)?;
    cmd.arg("install");
    cmd.arg("--global");
    cmd.arg("--user=root");
    cmd.arg("--cache=/tmp/npm-cache");
    cmd.args(&packages);
    run(cmd)
}

pub fn list(ctx: &mut Context) -> Result<(), StepError> {
    let path = Path::new("/vagga/container/npm-list.txt");
    let file = File::create(&path)
        .map_err(|e| StepError::Write(path.to_path_buf(), e))?;
    let mut cmd = command(ctx, &ctx.npm_settings.npm_exe)?;
    cmd.arg("ls");
    cmd.arg("--global");
    cmd.stdout(Stdio::from_file(file));
    run(cmd)
}

fn npm_hash_deps(data: &Json, key: &str, hash: &mut Digest) {
    let deps = data.find(key);
    if let Some(&Json::Object(ref ob)) = deps {
        // Note the BTree is sorted on its own
        for (key, val) in ob {
            hash.field(key, val.as_string().unwrap_or("*"));
        }
    }
}

impl BuildStep for NpmConfig {
    fn name(&self) -> &'static str { "NpmConfig" }
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        hash.field("npm_exe", &self.npm_exe);
        hash.field("install_node", self.install_node);
        Ok(())
    }
    fn build(&self, guard: &mut Guard, _build: bool)
        -> Result<(), StepError>
    {
        guard.ctx.npm_settings = self.clone();
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}

impl BuildStep for NpmInstall {
    fn name(&self) -> &'static str { "NpmInstall" }
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        hash.field("packages", &self.0);
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        guard.distro.npm_configure(&mut guard.ctx)?;
        if build {
            npm_install(&mut guard.distro, &mut guard.ctx, &self.0)?;
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}

impl BuildStep for NpmDependencies {
    fn name(&self) -> &'static str { "NpmDependencies" }
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        let path = Path::new("/work").join(&self.file);
        File::open(&path).map_err(|e| VersionError::Io(e, path.clone()))
        .and_then(|mut f| Json::from_reader(&mut f)
            .map_err(|e| VersionError::Json(e, path.to_path_buf())))
        .map(|data| {
            if self.package {
                npm_hash_deps(&data, "dependencies", hash);
            }
            if self.dev {
                npm_hash_deps(&data, "devDependencies", hash);
            }
            if self.peer {
                npm_hash_deps(&data, "peerDependencies", hash);
            }
            if self.bundled {
                npm_hash_deps(&data, "bundledDependencies", hash);
                npm_hash_deps(&data, "bundleDependencies", hash);
            }
            if self.optional {
                npm_hash_deps(&data, "optionalDependencies", hash);
            }
        })
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        guard.distro.npm_configure(&mut guard.ctx)?;
        if build {
            npm_deps(&mut guard.distro, &mut guard.ctx, &self)?;
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}
