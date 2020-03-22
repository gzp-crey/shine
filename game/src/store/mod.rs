/*/// Define engine limitations
pub mod libconfig {
    /// Maximum number of threads that can be used at once.
    /// Set to 0 find some "optimal" value based on the available number of logical cores.
    /// The allocated thread id's cannot exceed this hard limit, #see threadid.
    pub const MAX_THREAD_COUNT: usize = 0;

    /// Preferred number of threads used for data/task processing
    /// Set to 0 find some "optimal" value based on the available number of logical cores.
    pub const PREFERRED_THREAD_COUNT: usize = 0;
}*/

pub mod arena;
pub mod namedstore;
pub mod spscstate;
pub mod unnamedstore;
