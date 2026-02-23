use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cbf", version, about = "CBF developer workflow CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Bundle an application for distribution
    Bundle(BundleArgs),
}

#[derive(Debug, clap::Args)]
pub struct BundleArgs {
    #[command(subcommand)]
    pub command: BundleCommands,
}

#[derive(Debug, Subcommand)]
pub enum BundleCommands {
    /// Create a macOS .app bundle
    Macos(MacosBundleArgs),
}

#[derive(Debug, clap::Args)]
pub struct MacosBundleArgs {
    /// Path to built executable binary to bundle
    #[arg(long)]
    pub bin_path: PathBuf,

    /// Path to Chromium.app to bundle
    #[arg(long, env = "CBF_CHROMIUM_APP")]
    pub chromium_app: Option<PathBuf>,

    /// Directory containing libcbf_bridge.dylib
    #[arg(long, env = "CBF_BRIDGE_LIB_DIR")]
    pub bridge_lib_dir: Option<PathBuf>,

    /// Application display name
    #[arg(long)]
    pub app_name: Option<String>,

    /// CFBundleIdentifier
    #[arg(long)]
    pub bundle_identifier: Option<String>,

    /// Path to .icns file
    #[arg(long)]
    pub icon: Option<PathBuf>,

    /// Output directory where <AppName>.app is created
    #[arg(long, default_value = "dist")]
    pub out_dir: PathBuf,

    /// Cargo package name to load metadata from
    #[arg(long)]
    pub package: Option<String>,

    /// Code signing identity
    #[arg(long)]
    pub codesign_identity: Option<String>,
}
