use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "validate" => {
            let path = required_arg(&mut args, "path")?;
            validate_file(&path)?;
            println!("ok: {path} looks like a SkillSpec v0 file");
        }
        "test" => {
            let path = required_arg(&mut args, "path")?;
            validate_file(&path)?;
            println!("ok: scenario runner is not implemented yet");
            println!("next: parse tests[] and compare decide output");
        }
        "decide" => {
            let path = required_arg(&mut args, "path")?;
            let input = read_flag_value(&mut args, "--input")?;
            validate_file(&path)?;
            println!("{{");
            println!("  \"input\": {:?},", input);
            println!("  \"decision\": \"not_implemented\",");
            println!("  \"next\": \"implement rule matching for skillspec/v0\"");
            println!("}}");
        }
        "explain" => {
            let path = required_arg(&mut args, "path")?;
            let input = read_flag_value(&mut args, "--input")?;
            validate_file(&path)?;
            println!("SkillSpec explanation for {input:?}");
            println!("Rule matching is not implemented yet.");
        }
        "compile" => {
            let path = required_arg(&mut args, "path")?;
            let target = read_flag_value(&mut args, "--target")?;
            validate_file(&path)?;
            println!("ok: compiler target {target:?} is not implemented yet");
        }
        "import-skill" => {
            let path = required_arg(&mut args, "path")?;
            let out = read_flag_value(&mut args, "--out")?;
            import_skill(&path, &out)?;
            println!("ok: wrote initial structured scaffold to {out}");
        }
        "-h" | "--help" | "help" => print_help(),
        other => return Err(format!("unknown command {other:?}")),
    }

    Ok(())
}

fn validate_file(path: &str) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    for required in ["schema:", "id:", "title:"] {
        if !content.contains(required) {
            return Err(format!("{path} is missing required field {required}"));
        }
    }
    if !content.contains("schema: skillspec/v0") {
        return Err(format!("{path} must declare schema: skillspec/v0"));
    }
    Ok(())
}

fn import_skill(path: &str, out: &str) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    let title = content
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or("imported skill");
    let command_count = content
        .lines()
        .filter(|line| line.trim_start().starts_with("```"))
        .count()
        / 2;
    let heading_count = content.lines().filter(|line| line.starts_with('#')).count();
    let scaffold = format!(
        "schema: skillspec/v0\nid: imported.skill\ntitle: {:?}\ndescription: \"Imported scaffold from {}\"\n\nreview_required:\n  - \"Review extracted headings, command blocks, and always/never language.\"\n  - \"Add route rules, states, commands, and tests manually or with an agent-assisted pass.\"\n\nmetadata:\n  source: {:?}\n  heading_count: {}\n  command_block_count: {}\n\nrules: []\nstates: {{}}\ncommands: {{}}\ntests: []\n",
        title, path, path, heading_count, command_count
    );

    if let Some(parent) = Path::new(out).parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(out, scaffold).map_err(|error| format!("write {out}: {error}"))?;
    Ok(())
}

fn required_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {name}"))
}

fn read_flag_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    while let Some(arg) = args.next() {
        if arg == flag {
            return args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"));
        }
    }
    Err(format!("missing {flag}"))
}
fn print_help() {
    println!("skillspec - structured skills for agent behavior");
    println!();
    println!("usage:");
    println!("  skillspec validate <path>");
    println!("  skillspec test <path>");
    println!("  skillspec decide <path> --input <text>");
    println!("  skillspec explain <path> --input <text>");
    println!("  skillspec compile <path> --target <codex-skill|claude-skill|markdown>");
    println!("  skillspec import-skill <SKILL.md> --out <skill.spec.yml>");
}
