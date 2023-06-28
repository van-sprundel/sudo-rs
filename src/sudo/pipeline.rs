use std::process::exit;

use crate::cli::SudoOptions;
use crate::common::{Context, Environment, Error};
use crate::env::environment;
use crate::exec::ExitReason;
use crate::sudo::Duration;
use crate::sudoers::{Authorization, DirChange, Policy, PreJudgementPolicy};

pub trait PolicyPlugin {
    type PreJudgementPolicy: PreJudgementPolicy;
    type Policy: Policy;

    fn init(&mut self) -> Result<Self::PreJudgementPolicy, Error>;
    fn judge(
        &mut self,
        pre: Self::PreJudgementPolicy,
        context: &Context,
    ) -> Result<Self::Policy, Error>;
}

pub trait AuthPlugin {
    fn init(&mut self, context: &Context) -> Result<(), Error>;
    fn authenticate(
        &mut self,
        context: &Context,
        prior_validity: Duration,
        attempts: u16,
    ) -> Result<(), Error>;
    fn pre_exec(&mut self, context: &Context) -> Result<Environment, Error>;
    fn cleanup(&mut self);
}

pub struct Pipeline<Policy: PolicyPlugin, Auth: AuthPlugin> {
    pub policy: Policy,
    pub authenticator: Auth,
}

impl<Policy: PolicyPlugin, Auth: AuthPlugin> Pipeline<Policy, Auth> {
    pub fn run(&mut self, sudo_options: SudoOptions) -> Result<(), Error> {
        let pre = self.policy.init()?;
        let secure_path: String = pre
            .secure_path()
            .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
        let mut context = Context::build_from_options(sudo_options, secure_path)?;

        let policy = self.policy.judge(pre, &context)?;
        let authorization = policy.authorization();

        match authorization {
            Authorization::Forbidden => {
                return Err(Error::auth(&format!(
                    "I'm sorry {}. I'm afraid I can't do that",
                    context.current_user.name
                )));
            }
            Authorization::Allowed {
                must_authenticate,
                prior_validity,
                allowed_attempts,
            } => {
                self.apply_policy_to_context(&mut context, &policy)?;
                self.authenticator.init(&context)?;
                if must_authenticate {
                    self.authenticator
                        .authenticate(&context, prior_validity, allowed_attempts)?;
                }
            }
        }

        let additional_env = self.authenticator.pre_exec(&context)?;

        // build environment
        let current_env = std::env::vars_os().collect();
        let target_env =
            environment::get_target_environment(current_env, additional_env, &context, &policy);

        let pid = context.process.pid;

        // run command and return corresponding exit code
        let exec_result = if context.command.resolved {
            crate::exec::run_command(&context, target_env)
                .map_err(|io_error| Error::IoError(Some(context.command.command), io_error))
        } else {
            Err(Error::CommandNotFound(context.command.command))
        };

        self.authenticator.cleanup();

        let (reason, emulate_default_handler) = exec_result?;

        // Run any clean-up code before this line.
        emulate_default_handler();

        match reason {
            ExitReason::Code(code) => exit(code),
            ExitReason::Signal(signal) => {
                crate::system::kill(pid, signal)?;
            }
        }

        Ok(())
    }

    fn apply_policy_to_context(
        &mut self,
        context: &mut Context,
        policy: &<Policy as PolicyPlugin>::Policy,
    ) -> Result<(), crate::common::Error> {
        // see if the chdir flag is permitted
        match policy.chdir() {
            DirChange::Any => {}
            DirChange::Strict(optdir) => {
                if context.chdir.is_some() && context.chdir != std::env::current_dir().ok() {
                    return Err(Error::auth("no permission")); // TODO better user error messages
                } else {
                    context.chdir = optdir.map(std::path::PathBuf::from)
                }
            }
        }
        // override the default pty behaviour if indicated
        if !policy.use_pty() {
            context.use_pty = false
        }

        Ok(())
    }
}
