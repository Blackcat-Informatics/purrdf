#[unsafe(no_mangle)]
pub fn probe(values: &[u32]) -> u32 { values.iter().copied().sum() }
