use quire::validate as V;

use build_step::{BuildStep, VersionError, StepError, Digest, Config, Guard};


// Build Steps
#[derive(Debug)]
pub struct Install(Vec<String>);
tuple_struct_decode!(Install);

impl Install {
    pub fn config() -> V::Sequence<'static> {
        V::Sequence::new(V::Scalar::new())
    }
}

#[derive(Debug)]
pub struct BuildDeps(Vec<String>);
tuple_struct_decode!(BuildDeps);

impl BuildDeps {
    pub fn config() -> V::Sequence<'static> {
        V::Sequence::new(V::Scalar::new())
    }
}

#[derive(Debug)]
pub struct Repo(String);
tuple_struct_decode!(Repo);

impl Repo {
    pub fn config() -> V::Scalar {
        V::Scalar::new()
    }
}

impl BuildStep for Install {
    fn name(&self) -> &'static str { "Install" }
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        hash.field("packages", &self.0);
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        guard.ctx.packages.extend(self.0.clone().into_iter());
        for i in self.0.iter() {
            guard.ctx.build_deps.remove(i);
        }
        if build {
            guard.distro.install(&mut guard.ctx, &self.0)?;
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}

impl BuildStep for BuildDeps {
    fn name(&self) -> &'static str { "BuildDeps" }
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        hash.field("packages", &self.0);
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        if build {
            for i in self.0.iter() {
                if !guard.ctx.packages.contains(i) {
                    guard.ctx.build_deps.insert(i.clone());
                }
            }
            guard.distro.install(&mut guard.ctx, &self.0)?;
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}

impl BuildStep for Repo {
    fn name(&self) -> &'static str { "Repo" }
    fn hash(&self, _cfg: &Config, hash: &mut Digest)
        -> Result<(), VersionError>
    {
        hash.field("name", &self.0);
        Ok(())
    }
    fn build(&self, guard: &mut Guard, build: bool)
        -> Result<(), StepError>
    {
        if build {
            guard.distro.add_repo(&mut guard.ctx, &self.0)?;
        }
        Ok(())
    }
    fn is_dependent_on(&self) -> Option<&str> {
        None
    }
}
