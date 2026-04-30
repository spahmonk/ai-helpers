pub mod cli;
pub mod contracts;
// pub mod mcp;  // TODO: fix MCP adapter - has pre-existing compilation errors

use self::contracts::{DoctorService, ReadService, SearchService, ShellService, TreeService};
use crate::core::config::AppConfig;

/// Shared app wiring is generic over service implementations, but each slot must
/// satisfy the shared service contract.
///
/// ```compile_fail
/// use ctx_lite::app::AppServices;
///
/// let _services = AppServices::new((), (), (), (), ());
/// ```
pub struct AppServices<Read, Tree, Search, Shell, Doctor>
where
    Read: ReadService,
    Tree: TreeService,
    Search: SearchService,
    Shell: ShellService,
    Doctor: DoctorService,
{
    pub read: Read,
    pub tree: Tree,
    pub search: Search,
    pub shell: Shell,
    pub doctor: Doctor,
}

impl<Read, Tree, Search, Shell, Doctor> AppServices<Read, Tree, Search, Shell, Doctor>
where
    Read: ReadService,
    Tree: TreeService,
    Search: SearchService,
    Shell: ShellService,
    Doctor: DoctorService,
{
    pub fn new(read: Read, tree: Tree, search: Search, shell: Shell, doctor: Doctor) -> Self {
        Self {
            read,
            tree,
            search,
            shell,
            doctor,
        }
    }
}

pub struct AppWorkflow<Read, Tree, Search, Shell, Doctor>
where
    Read: ReadService,
    Tree: TreeService,
    Search: SearchService,
    Shell: ShellService,
    Doctor: DoctorService,
{
    pub config: AppConfig,
    pub services: AppServices<Read, Tree, Search, Shell, Doctor>,
}

impl<Read, Tree, Search, Shell, Doctor> AppWorkflow<Read, Tree, Search, Shell, Doctor>
where
    Read: ReadService,
    Tree: TreeService,
    Search: SearchService,
    Shell: ShellService,
    Doctor: DoctorService,
{
    pub fn new(
        config: AppConfig,
        services: AppServices<Read, Tree, Search, Shell, Doctor>,
    ) -> Self {
        Self { config, services }
    }
}
