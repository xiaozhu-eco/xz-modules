//! Domain-specific memory abstractions.
//!
//! These modules provide higher-level interfaces for storing and retrieving
//! structured domain data (characters, plot arcs, world state, etc.) via the
//! underlying [`MemorySystem`](crate::MemorySystem) trait.

pub mod character;
pub mod plot;
pub mod seed;
pub mod style;

pub use character::CharacterMemory;
pub use plot::PlotMemory;
pub use seed::SeedMemory;
pub use style::StyleMemory;
