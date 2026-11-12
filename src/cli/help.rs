use super::command::{COMMANDS, OPTIONS, ValueMode};

fn options(command: Option<&str>, common: bool) -> String {
    let mut out = String::new();
    for spec in OPTIONS.iter().filter(|s| {
        if common { s.commands.is_empty() } else { command.is_some_and(|c| s.commands.contains(&c)) }
    }) {
        let mut spelling = spec.short.map(|c| format!("-{c}, ")).unwrap_or_else(|| "    ".into());
        spelling.push_str("--");
        spelling.push_str(spec.long);
        match spec.value {
            ValueMode::Required(name) => spelling.push_str(&format!(" <{name}>")),
            ValueMode::Optional(name) => spelling.push_str(&format!(" [{name}]")),
            ValueMode::Flag => {}
        }
        out.push_str(&format!("  {spelling:30} {}", spec.help));
        if let Some(alias) = spec.alias { out.push_str(&format!(" [alias: --{alias}]")); }
        if spec.repeatable { out.push_str(" [repeatable]"); }
        out.push('\n');
    }
    out
}

pub fn render(command: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(name) = command {
        let spec = COMMANDS.iter().find(|c| c.name == name).expect("registered command");
        out.push_str(&format!("{}\n\nUsage: rimage {} [OPTIONS] <FILES>...\n\nOptions:\n", spec.help, spec.name));
        out.push_str(&options(Some(name), false));
        out.push_str(&options(Some(name), true));
    } else {
        out.push_str("Usage: rimage <COMMAND> [OPTIONS] <FILES>...\n\nCommon options:\n");
        out.push_str(&options(None, true));
        out.push_str("\nCommands:\n");
        for spec in COMMANDS {
            out.push_str(&format!("\n  {} — {}\n  Usage: rimage {} [OPTIONS] <FILES>...\n", spec.name, spec.help, spec.name));
            if let Some(alias) = spec.alias { out.push_str(&format!("  Alias: {alias}\n")); }
            out.push_str("  Common options are inherited.\n");
            out.push_str(&options(Some(spec.name), false));
        }
    }
    out
}
