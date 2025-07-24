use super::BootstrapMode;
use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct Bootstrap {
    /// Bootstrap mode: 'minimal' or 'full'
    pub mode: BootstrapMode,
}
