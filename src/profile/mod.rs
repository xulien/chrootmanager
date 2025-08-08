//! Profile management module for Gentoo architecture profiles
//!
//! This module provides functionality to discover and manage different architecture
//! profiles available on Gentoo mirrors without hardcoded enums.

use crate::profile::architecture::Architecture;

pub mod parser;
mod architecture;
pub(crate) mod manager;
pub(crate) mod selected;

