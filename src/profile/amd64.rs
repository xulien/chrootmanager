use std::fmt::{Display, Formatter};
use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, EnumIter)]
pub enum Amd64Profile {
    DesktopOpenrc,
    DesktopSystemd,
    HardenedSelinuxOpenrc,
    HardenedOpenrc,
    HardenedSystemd,
    LlvmOpenrc,
    LlvmSystemd,
    MuslHardened,
    MuslLlvm,
    Musl,
    NoMultilibOpenrc,
    NoMultilibSystemd,
    OpenrcSplitusr,
    Openrc,
    Systemd,
    X32Openrc,
    X32Systemd,
}

impl Display for Amd64Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Amd64Profile::DesktopOpenrc => write!(f, "desktop-openrc"),
            Amd64Profile::DesktopSystemd => write!(f, "desktop-systemd"),
            Amd64Profile::HardenedSelinuxOpenrc => write!(f, "hardened-selinux-openrc"),
            Amd64Profile::HardenedOpenrc => write!(f, "hardened-openrc"),
            Amd64Profile::HardenedSystemd => write!(f, "hardened-systemd"),
            Amd64Profile::LlvmOpenrc => write!(f, "llvm-openrc"),
            Amd64Profile::LlvmSystemd => write!(f, "llvm-systemd"),
            Amd64Profile::MuslHardened => write!(f, "musl-hardened"),
            Amd64Profile::MuslLlvm => write!(f, "musl-llvm"),
            Amd64Profile::Musl => write!(f, "musl"),
            Amd64Profile::NoMultilibOpenrc => write!(f, "no-multilib-openrc"),
            Amd64Profile::NoMultilibSystemd => write!(f, "no-multilib-systemd"),
            Amd64Profile::OpenrcSplitusr => write!(f, "openrc-splitusr"),
            Amd64Profile::Openrc => write!(f, "openrc"),
            Amd64Profile::Systemd => write!(f, "systemd"),
            Amd64Profile::X32Openrc => write!(f, "x32-openrc"),
            Amd64Profile::X32Systemd => write!(f, "x32-systemd"),
        }
    }
}

impl Default for Amd64Profile {
    fn default() -> Self {
        Self::Openrc
    }
}