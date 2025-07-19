use crate::error::ChrootManagerError;
use crate::profile::amd64::Amd64Profile;
use crate::profile::arm64::Arm64Profile;

use crate::error::ChrootManagerError::System;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::process::Command;
use strum::{EnumIter, IntoEnumIterator};

pub struct ProfileLink(pub String);
impl ProfileLink {
    pub fn get(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, EnumIter)]
pub enum Arch {
    Amd64(Amd64Profile),
    Arm64(Arm64Profile),
}

impl Display for Arch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Amd64(flavor) => match flavor {
                Amd64Profile::DesktopOpenrc => write!(f, "amd64-desktop-openrc"),
                Amd64Profile::DesktopSystemd => write!(f, "amd64-desktop-systemd"),
                Amd64Profile::HardenedSelinuxOpenrc => write!(f, "amd64-hardened-selinux-openrc"),
                Amd64Profile::HardenedOpenrc => write!(f, "amd64-hardened-openrc"),
                Amd64Profile::HardenedSystemd => write!(f, "amd64-hardened-systemd"),
                Amd64Profile::LlvmOpenrc => write!(f, "amd64-llvm-openrc"),
                Amd64Profile::LlvmSystemd => write!(f, "amd64-llvm-systemd"),
                Amd64Profile::MuslHardened => write!(f, "amd64-musl-hardened"),
                Amd64Profile::MuslLlvm => write!(f, "amd64-musl-llvm"),
                Amd64Profile::Musl => write!(f, "amd64-musl"),
                Amd64Profile::NoMultilibOpenrc => write!(f, "amd64-no-multilib-openrc"),
                Amd64Profile::NoMultilibSystemd => write!(f, "amd64-no-multilib-systemd"),
                Amd64Profile::OpenrcSplitusr => write!(f, "amd64-openrc-splitusr"),
                Amd64Profile::Openrc => write!(f, "amd64-openrc"),
                Amd64Profile::Systemd => write!(f, "amd64-systemd"),
                Amd64Profile::X32Openrc => write!(f, "amd64-x32-openrc"),
                Amd64Profile::X32Systemd => write!(f, "amd64-x32-systemd"),
            },
            Self::Arm64(flavor) => match flavor {
                Arm64Profile::Aarch64beOpenrc => write!(f, "aarch64_be-openrc"),
                Arm64Profile::Aarch64beSystemd => write!(f, "aarch64_be-systemd"),
                Arm64Profile::DesktopOpenrc => write!(f, "arm64-desktop-openrc"),
                Arm64Profile::DesktopSystemd => write!(f, "arm64-desktop-systemd"),
                Arm64Profile::LlvmOpenrc => write!(f, "arm64-llvm-openrc"),
                Arm64Profile::LlvmSystemd => write!(f, "arm64-llvm-systemd"),
                Arm64Profile::MuslHardened => write!(f, "arm64-musl-hardened"),
                Arm64Profile::MuslLlvm => write!(f, "arm64-musl-llvm"),
                Arm64Profile::Musl => write!(f, "arm64-musl"),
                Arm64Profile::OpenrcSplitusr => write!(f, "arm64-openrc-splitusr"),
                Arm64Profile::Openrc => write!(f, "arm64-openrc"),
                Arm64Profile::Systemd => write!(f, "arm64-systemd"),
            },
        }
    }
}

impl From<String> for Arch {
    fn from(s: String) -> Self {
        Arch::from(ProfileLink(s))
    }
}

impl Arch {
    pub(crate) fn arch(&self) -> &str {
        match self {
            Arch::Amd64(_) => "amd64",
            Arch::Arm64(_) => "arm64",
        }
    }

    pub fn labels() -> Vec<String> {
        Self::iter()
            .map(|p| p.arch().to_string())
            .collect::<Vec<String>>()
    }
    pub fn read_fs(path: &Path) -> Result<Arch, ChrootManagerError> {
        let profile_link_path = path.join("etc").join("portage").join("make.profile");

        let output = Command::new("readlink")
            .arg(format!("{}", profile_link_path.display()))
            .output()
            .map_err(|e| {
                System(format!(
                    "Error reading profile symbolic link {} : {e}",
                    profile_link_path.display()
                ))
            })?;

        if output.status.success() {
            log::info!("Profile reading successful");
            match output.stdout.is_empty() {
                true => Err(System("Error reading profile, output is empty".to_string())),
                false => {
                    let out = String::from_utf8_lossy(&output.stdout).to_string();
                    log::info!("out : {out}");
                    Ok(Arch::from(out))
                }
            }
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(System(format!(
                "Error reading profile {}: {error_msg}",
                profile_link_path.display()
            )))
        }
    }
}

impl From<ProfileLink> for Arch {
    fn from(profile_link: ProfileLink) -> Self {
        let mut v = profile_link
            .get()
            .trim_end()
            .split("linux/")
            .collect::<Vec<&str>>()[1]
            .split("/")
            .collect::<Vec<&str>>();
        v.remove(1);
        let s = v.join("-");
        match s.as_str() {
            "amd64-desktop" | "amd64-desktop-gnome" | "amd64-desktop-plasma" => {
                Arch::Amd64(Amd64Profile::DesktopOpenrc)
            }
            "amd64-desktop-systemd"
            | "amd64-desktop-gnome-systemd"
            | "amd64-desktop-plasma-systemd" => Arch::Amd64(Amd64Profile::DesktopSystemd),
            "amd64-systemd" => Arch::Amd64(Amd64Profile::Systemd),
            "amd64" => Arch::Amd64(Amd64Profile::Openrc),
            "amd64-split-usr" => Arch::Amd64(Amd64Profile::OpenrcSplitusr),
            "amd64-hardened" => Arch::Amd64(Amd64Profile::HardenedOpenrc),
            "amd64-hardened-systemd" => Arch::Amd64(Amd64Profile::HardenedSystemd),
            "amd64-hardened-selinux" => {
                Arch::Amd64(Amd64Profile::HardenedSelinuxOpenrc)
            }
            _ => panic!("Profile link could not be parsed {s}"),
        }
    }
}
