//! Agent-ready noun-verb CLI over the typed planner library.
//!
//! Command wrappers are manufactured from the repository ontology by ggen.
//! Domain behavior remains in `handlers`; generated wrappers stay thin.

mod handlers;
mod plan;

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
