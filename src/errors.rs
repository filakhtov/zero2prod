pub fn error_chain_fmt(
    e: &dyn std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let current = e.source();

    if let Some(cause) = current {
        writeln!(f, "Caused by:\n\t")?;
        error_chain_fmt(cause, f)?;
    }

    Ok(())
}
