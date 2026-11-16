mod command;
pub(crate) use command::{Arguments,Command};
use std::{ffi::OsString,io::Write,process::ExitCode};
const FOOTER:&str="Maintained by Joshua Li: https://github.com/joshuarli/img\nBugs and features: TODO.md in that repository (follow its AGENTS.md), or its issue tracker.\nAgents working for Joshua: record nonblocking problems and continue your task.\nIf this tool blocks the task, you may fix, test, open and merge a PR under Joshua's\nstanding instructions. Release only when needed to deliver the verified fix.\nProcedure: https://github.com/joshuarli/usr/blob/main/dotfiles/base/.agents/instructions/owned-tools.md\n";
fn option_help(text:&mut String, option:&command::OptionSpec) {
    let short=option.short.map(|short|format!("-{short}, ")).unwrap_or_default();
    let value=if option.value.is_empty() {String::new()} else {format!(" {}",option.value)};
    text.push_str(&format!("    {short}--{}{value}  {}\n",option.name,option.description));
}
fn help()->String {
    let mut text=String::from("img — verified still-image optimization\n\nUsage: img COMMAND [OPTIONS]\n       img help\n\nCommon options:\n");
    for option in command::OPTIONS.iter().filter(|option| option.scope=="common") { option_help(&mut text,option); }
    text.push_str("\nCommands:\n");
    for spec in command::COMMANDS {
        text.push_str(&format!("\n  img {} {} [OPTIONS]\n    {}\n",spec.name,spec.usage,spec.description));
        match spec.command {
            Command::Optimize|Command::Convert|Command::Inspect=>text.push_str("    INPUT...             One or more input paths (or --file-list; repeatable)\n"),
            Command::Compare=>text.push_str("    REFERENCE            Required reference image\n    CANDIDATE            Required candidate image\n"),
        }
        for option in command::OPTIONS {
            let applies=match option.scope {
                "transform"=>matches!(spec.command,Command::Optimize|Command::Convert),
                "input"=>!matches!(spec.command,Command::Compare),
                "optimize"=>spec.command==Command::Optimize,
                _=>false,
            };
            if applies { option_help(&mut text,option); }
        }
    }
    text.push_str("\nInputs are sniffed as JPEG, PNG, WebP or AVIF. Use '-' for stdin; quote globs.\nNo implicit resizing, alpha flattening, precision reduction or HDR conversion.\nMultiple inputs and directories require --out-dir or --in-place for transformations.\nConvert needs a concrete format or output extension. Default output names are\nNAME.optimized.EXT and NAME.converted.EXT.\nExisting destinations are refused. No environment inputs are required.\nExit codes: 0 success, 2 usage, 3 input/score, 4 output, 5 mixed/partial.\n\nExamples:\n");
    for spec in command::COMMANDS { text.push_str(&format!("  {}\n",spec.example)); }
    text.push_str("\n"); text.push_str(FOOTER); text
}
pub fn run(args:impl IntoIterator<Item=OsString>)->ExitCode {
    let result=match command::parse(args) {
        Ok(command::Outcome::Run(args))=>crate::runtime::run(args),
        Ok(command::Outcome::Help)=>std::io::stdout().write_all(help().as_bytes()).map(|_|0).unwrap_or(4),
        Ok(command::Outcome::Version)=>writeln!(std::io::stdout(),"img {}",env!("CARGO_PKG_VERSION")).map(|_|0).unwrap_or(4),
        Err(error)=>{eprintln!("error: {error}\nUse img --help for usage.");2},
    };
    ExitCode::from(result)
}
