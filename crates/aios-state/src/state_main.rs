//! Fixed private graph owner/control entry point. No arbitrary path or SQL.
fn main(){if let Err(error)=aios_state::graph::runtime::entry(){eprintln!("aios-stated: refused: {error:?}");std::process::exit(1);}}
