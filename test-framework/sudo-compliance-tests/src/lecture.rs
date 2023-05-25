use sudo_test::{Command, Env, User};
use crate::{Result, SUDOERS_ROOT_ALL, USERNAME, SUDOERS_USER_ALL_ALL, SUDOERS_ALWAYS_LECTURE, SUDOERS_NO_LECTURE, PASSWORD, OG_SUDO_STANDARD_LECTURE};

#[test]
#[ignore]
fn default_lecture_shown_once() -> Result<()> {
    let env = Env([SUDOERS_ROOT_ALL, SUDOERS_USER_ALL_ALL])
        .user(User(USERNAME).password(PASSWORD))
        .build()?;

    let output = Command::new("sudo")
    .args(["-S", "true"])
    .as_user(USERNAME)
    .stdin(PASSWORD)
    .exec(&env)?;
    assert_eq!(true, output.status().success());

    assert_contains!(
        output.stderr(),
        OG_SUDO_STANDARD_LECTURE
    );

    let second_sudo = Command::new("sudo")
    .as_user(USERNAME)
    .stdin(PASSWORD)
    .args(["-S", "echo", "Yeah!"])
    .exec(&env)?;

    assert_eq!(true, second_sudo.status().success());
    assert_ne!(second_sudo.stdout().unwrap(), OG_SUDO_STANDARD_LECTURE);
    Ok(())
}

#[test]
#[ignore]
fn lecture_always_shown() -> Result<()> {
    let env = Env([
        SUDOERS_ROOT_ALL,
        SUDOERS_ALWAYS_LECTURE
        ])
        .user(User(USERNAME).password(PASSWORD)).build()?;

    let output = Command::new("sudo")
    .as_user(USERNAME)
    .stdin(PASSWORD)
    .args(["-S", "true"])
    .exec(&env)?;
    assert_eq!(false, output.status().success());

    assert_contains!(
        output.stderr(),
        OG_SUDO_STANDARD_LECTURE
    );

    let second_sudo = Command::new("sudo")
    .as_user(USERNAME)
    .stdin(PASSWORD)
    .args(["-S", "ls"])
    .exec(&env)?;
    assert_eq!(false, output.status().success());

    assert_contains!(
        second_sudo.stderr(),
        OG_SUDO_STANDARD_LECTURE
    );
    Ok(())
}

#[test]
fn lecture_never_shown() -> Result<()> {
    let env = Env([SUDOERS_ROOT_ALL, SUDOERS_USER_ALL_ALL, SUDOERS_NO_LECTURE])
        .user(User(USERNAME).password(PASSWORD))
        .build()?;

    let output = Command::new("sudo")
    .as_user(USERNAME)
    .stdin(PASSWORD)
    .args(["-S", "echo", "Yeah!"])
    .exec(&env)?;

    assert_eq!(true, output.status().success());
    assert_ne!(output.stdout().unwrap(), OG_SUDO_STANDARD_LECTURE);
    Ok(())
}

#[test]
fn negation_equals_never() -> Result<()> {
    let env = Env([SUDOERS_ROOT_ALL, SUDOERS_USER_ALL_ALL, "Defaults  !lecture"])
        .user(User(USERNAME).password(PASSWORD))
        .build()?;

    let output = Command::new("sudo")
    .as_user(USERNAME)
    .stdin(PASSWORD)
    .args(["-S", "echo", "Yeah!"])
    .exec(&env)?;

    assert_eq!(true, output.status().success());
    assert_ne!(output.stdout().unwrap(), OG_SUDO_STANDARD_LECTURE);
    Ok(())
}
