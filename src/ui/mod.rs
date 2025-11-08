use console::{style, Term};

pub fn section(title: &str) {
    let term = Term::stdout();
    let line = "━".repeat(40);
    let _ = term.write_line(&format!("{}\n{}\n{}", style(&line).dim(), style(title).bold(), style(&line).dim()));
}

pub fn step(message: &str) {
    let term = Term::stdout();
    let _ = term.write_line(&format!("{} {}", style("→").bold(), message));
}

pub fn success(message: &str) {
    let term = Term::stdout();
    let _ = term.write_line(&format!("{} {}", style("✔").green().bold(), message));
}

pub fn info(message: &str) {
    let term = Term::stdout();
    let _ = term.write_line(&format!("{} {}", style("i").cyan().bold(), message));
}

pub fn error(message: &str) {
    let term = Term::stderr();
    let _ = term.write_line(&format!("{} {}", style("x").red().bold(), message));
}


