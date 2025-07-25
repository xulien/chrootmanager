use std::fmt::{Display, Formatter};
use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, EnumIter)]
pub enum Arm64Profile {
    Aarch64beOpenrc,
    Aarch64beSystemd,
    DesktopOpenrc,
    DesktopSystemd,
    LlvmOpenrc,
    LlvmSystemd,
    MuslHardened,
    MuslLlvm,
    Musl,
    OpenrcSplitusr,
    Openrc,
    Systemd,
}

impl Display for Arm64Profile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Arm64Profile::Aarch64beOpenrc => write!(f, "aarch64be-openrc"),
            Arm64Profile::Aarch64beSystemd => write!(f, "aarch64be-systemd"),
            Arm64Profile::DesktopOpenrc => write!(f, "desktop-openrc"),
            Arm64Profile::DesktopSystemd => write!(f, "desktop-systemd"),
            Arm64Profile::LlvmOpenrc => write!(f, "llvm-openrc"),
            Arm64Profile::LlvmSystemd => write!(f, "llvm-systemd"),
            Arm64Profile::MuslHardened => write!(f, "musl-hardened"),
            Arm64Profile::MuslLlvm => write!(f, "musl-llvm"),
            Arm64Profile::Musl => write!(f, "musl"),
            Arm64Profile::OpenrcSplitusr => write!(f, "openrc-splitusr"),
            Arm64Profile::Openrc => write!(f, "openrc"),
            Arm64Profile::Systemd => write!(f, "systemd"),
        }
    }
}

impl Default for Arm64Profile {
    fn default() -> Self {
        Self::Openrc
    }
}
