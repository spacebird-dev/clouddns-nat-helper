use clouddns_nat_helper::{
    ipv4source::{Ipv4Source, SourceError},
    plan::{Action, Plan},
    provider::{Provider, ProviderError},
    registry::{ARegistry, RegistryError},
};
use log::{debug, info};
use thiserror::Error;

use crate::cli::Policy;

/// An executor performs the complete set of actions needed to bring our records up-to-date
pub struct Executor<'a> {
    source: &'a dyn Ipv4Source,
    provider: &'a mut dyn Provider,
    registry: &'a mut dyn ARegistry,
    policy: Policy,
}

#[derive(Error, Debug, Eq, PartialEq, Clone)]
pub enum ExecutorError {
    #[error("`{0}`")]
    Provider(ProviderError),
    #[error("`{0}`")]
    Registry(RegistryError),
    #[error("`{0}`")]
    Source(SourceError),
}
impl From<ProviderError> for ExecutorError {
    fn from(p: ProviderError) -> Self {
        ExecutorError::Provider(p)
    }
}
impl From<RegistryError> for ExecutorError {
    fn from(r: RegistryError) -> Self {
        ExecutorError::Registry(r)
    }
}
impl From<SourceError> for ExecutorError {
    fn from(s: SourceError) -> Self {
        ExecutorError::Source(s)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RunResult {
    pub successes: Vec<Action>,
    pub failures: Vec<(Action, ExecutorError)>,
}

impl<'a> Executor<'a> {
    /// Create a new basic executor
    pub fn try_new(
        source: &'a dyn Ipv4Source,
        provider: &'a mut dyn Provider,
        registry: &'a mut dyn ARegistry,
        policy: Policy,
        dry_run: bool,
    ) -> Result<Executor<'a>, ExecutorError> {
        if dry_run {
            provider.enable_dry_run()?;
            registry.enable_dry_run()?;
        }
        Ok(Self {
            source,
            provider,
            registry,
            policy,
        })
    }

    pub fn run(&mut self) -> Result<RunResult, ExecutorError> {
        let target_addr = match self.source.addr() {
            Ok(a) => a,
            Err(e) => return Err(e.into()),
        };
        info!("Target Ipv4 address: {}", target_addr);

        info!("Generating plan and registering domains...");
        let plan = Plan::generate(self.registry, target_addr, self.policy.into());
        debug!("Generated plan: {:?}", plan);

        let mut successes: Vec<Action> = vec![];
        let mut failures: Vec<(Action, ExecutorError)> = vec![];

        for action in plan.actions() {
            match action {
                Action::ClaimAndUpdate(domain, _) => {
                    match self.registry.claim(domain.as_str()) {
                        Ok(_) => {}
                        Err(e) => {
                            failures.push((action.clone(), e.into()));
                            continue;
                        }
                    };
                    match self.provider.apply(action) {
                        Ok(_) => {
                            successes.push(action.clone());
                        }
                        Err(e) => failures.push((action.clone(), e.into())),
                    };
                }
                Action::Update(_, _) => {
                    match self.provider.apply(action) {
                        Ok(_) => {
                            successes.push(action.clone());
                        }
                        Err(e) => failures.push((action.clone(), e.into())),
                    };
                }
                Action::DeleteAndRelease(domain) => {
                    match self.provider.apply(action) {
                        Ok(_) => {}
                        Err(e) => failures.push((action.clone(), e.into())),
                    };
                    match self.registry.release(domain) {
                        Ok(_) => {
                            successes.push(action.clone());
                        }
                        Err(e) => failures.push((action.clone(), e.into())),
                    };
                }
                _ => todo!(),
            }
        }
        Ok(RunResult {
            successes,
            failures,
        })
    }
}
