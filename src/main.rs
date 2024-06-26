use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::io::Write;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::process::{Command, exit, Stdio};
use std::thread::available_parallelism;
use clap::Parser;
use itertools::Itertools;
use lalrpop_util::ParseError;
use minos::{Label, Report, ReportKind, Source};
use ast::{ReactionTerms, Symbol};
use crate::ast::{Goal, Program, Target};

mod grammar;
mod ast;

const MINIZINC_OUTPUT_NAME: &str = "program.mzn";

pub fn merge_terms<'s>(a: ReactionTerms<'s>, b: ReactionTerms<'s>) -> ReactionTerms<'s> {
    let mut res = HashMap::new();

    for m in [a, b] {
        for (k, v) in m {
            *res.entry(k).or_insert(0) += v;
        }
    }

    res
}

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// the chem file to work on
    #[arg(value_name = "FILE", env="FILE")]
    file: PathBuf,

    /// The target to optimize
    #[arg(value_name = "TARGET", env="TARGET")]
    target: String,

    /// Arguments to give to the solver (through minizinc)
    #[arg(long, short, value_name = "SOLVER_ARGS", env="SOLVER_ARGS")]
    solver_arguments: Option<String>
}

fn exit_report(r: &Report, source: Source) -> ! {
    r.eprint(source).expect("io error");
    exit(1);
}

fn main() {
    let args = Cli::parse();
    let input = match read_to_string(&args.file) {
        Ok(i) => i,
        Err(e) => {
            let name = args.file.to_string_lossy();
            exit_report(
                &Report::build(ReportKind::Error)
                    .with_message(e.to_string())
                    .with_label(Label::new(0..name.chars().count()).with_message("while reading this file"))
                    .finish(),
                Source::from(name.to_string())
            );
        }
    };

    let program = parse(&input, args.file.to_string_lossy().as_ref());

    let Some(target ) = program.targets.get(args.target.as_str()) else {
        let cmdline_args = std::env::args().join(" ");
        let offset = cmdline_args.find(&args.target).unwrap();

        exit_report(
            &Report::build(ReportKind::Error)
                .with_message("target name not found".to_string())
                .with_label(
                    Label::new(offset..offset+args.target.chars().count()).with_message(format!("'{}' not found", args.target))
                )
                .with_help(format!("did you mean {}", expected_str("", program.targets.into_keys())))
                .finish(),
            Source::from(cmdline_args)
        );
    };

    let mut f = match File::create(MINIZINC_OUTPUT_NAME) {
        Ok(f) => f,
        Err(e) => {
            exit_report(
                &Report::build(ReportKind::Error)
                    .with_message(format!("{e}"))
                    .with_label(Label::new(0..MINIZINC_OUTPUT_NAME.chars().count()).with_message("while creating this file"))
                    .finish(),
                Source::from(MINIZINC_OUTPUT_NAME.to_string())
            );
        }
    };

    if let Err(e) = generate_minizinc(&mut f, &input, &program, target) {
        exit_report(
            &Report::build(ReportKind::Error)
                .with_message(e.to_string())
                .with_label(Label::new(0..MINIZINC_OUTPUT_NAME.chars().count()).with_message("while writing to this file"))
                .finish(),
            Source::from(MINIZINC_OUTPUT_NAME.to_string())
        );
    }

    drop(f);

    let cpus = available_parallelism().expect("get available parallelism").to_string();

    let mut cmd = Command::new("minizinc");
    cmd
        .args(["--soln-sep", ""])
        .args(["--search-complete-msg", ""])
        .args(["--unsatorunbnd-msg", "unsatisfiable or unbounded"])
        .args(["--unsatisfiable-msg", "unsatisfiable"])
        .args(["--solver", "cbc"])
        .args(["-p", cpus.as_str()])
        .arg(MINIZINC_OUTPUT_NAME)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());


    let output = match cmd.output() {
        Ok(child) => child,
        Err(e) => {
            exit_report(
                &Report::build(ReportKind::Error)
                    .with_message(format!("while spawning 'minizinc' process: {e}"))
                    .finish(),
                Source::from("minizinc".to_string())
            );
        }
    };

    if !output.status.success() {
        let output = String::from_utf8_lossy(&output.stderr).to_string();
        exit_report(
            &Report::build(ReportKind::Error)
                .with_message("while running 'minizinc' process".to_string())
                .with_code(&output)
                .finish(),
            Source::from(output)
        );
    }

    println!("{}", String::from_utf8_lossy(&output.stdout));
}

fn generate_minizinc(w: &mut impl Write, input: &str, program: &Program, target: &Target) -> io::Result<()> {
    let Some(ref goal) = target.goal else {
        exit_report(
            &Report::build(ReportKind::Error)
                .with_message(format!("expected 'goal' specification in target {}", target.name))
                .with_label(Label::new(target.span.0..target.span.1).with_message("in this target"))
                .finish(),
            Source::from(input.to_string())
        );
    };

    for i in input.lines() {
        writeln!(w, "% {i}")?;
    }

    writeln!(w)?;
    writeln!(w, "% variables")?;

    for reaction in &program.reactions {
        let variable = reaction.var_name();
        writeln!(w, "var float: {variable};")?;
    }

    writeln!(w)?;
    writeln!(w, "% non-negative constraints")?;
    for reaction in &program.reactions {
        writeln!(w, "constraint {} >= 0;", reaction.var_name())?;
    }

    writeln!(w)?;
    writeln!(w, "% target constraints")?;
    for (symbol, scalar) in &target.constraints {
        let mut production = vec!["0".to_string()];
        let mut consumption = vec!["0".to_string()];

        for reaction in &program.reactions {
            let cost = reaction.cost.0;

            if let Some(&i) = reaction.inputs.get(symbol) {
                consumption.push(format!("{i} * {} / {cost}", reaction.var_name()))
            }

            if let Some(&i) = reaction.outputs.get(symbol) {
                production.push(format!("{i} * {} / {cost}", reaction.var_name()))
            }
        }

        let production = production.join("+");
        let consumption = consumption.join("+");

        let in_time = target.in_time;

        writeln!(w, "constraint ({production}) - ({consumption}) >= {scalar} / {in_time};")?;
    }

    writeln!(w)?;
    writeln!(w, "% balance constraints")?;

    let mut symbols: HashSet<Symbol> = HashSet::new();
    for i in &program.reactions {
        symbols.extend(i.inputs.keys());
        symbols.extend(i.outputs.keys());
    }

    let using: HashSet<&Symbol> = target.inputs.iter().collect();
    for symbol in symbols {
        match goal {
            Goal::Reactions if using.contains(&symbol) => continue,
            Goal::Resources(rt) if using.contains(&symbol) || rt.keys().contains(&symbol) => continue,
            _ => {}
        }

        let mut production = vec!["0".to_string()];
        let mut consumption = vec!["0".to_string()];

        for reaction in &program.reactions {
            let cost = reaction.cost.0;

            if let Some(&i) = reaction.inputs.get(&symbol) {
                consumption.push(format!("{i} * {} / {cost}", reaction.var_name()))
            }

            if let Some(&i) = reaction.outputs.get(&symbol) {
                production.push(format!("{i} * {} / {cost}", reaction.var_name()))
            }
        }

        let production = production.join("+");
        let consumption = consumption.join("+");

        writeln!(w, "constraint ({production}) >= {consumption};")?;
    }

    writeln!(w)?;
    match goal {
        Goal::Resources(rt) => {
            let mut production = vec!["0".to_string()];
            let mut consumption = vec!["0".to_string()];

            for (symbol, weight) in rt {
                for reaction in &program.reactions {
                    if let Some(&i) = reaction.inputs.get(&symbol) {
                        consumption.push(format!("{i} * {} * {weight}", reaction.var_name()))
                    }

                    if let Some(&i) = reaction.outputs.get(&symbol) {
                        production.push(format!("{i} * {} * {weight}", reaction.var_name()))
                    }
                }
            }

            let production = production.join("+");
            let consumption = consumption.join("+");

            writeln!(w, "solve minimize ({consumption}) - ({production});")?;
        }
        Goal::Reactions => {
            writeln!(w, "solve minimize {};", program.reactions.iter().map(|i| i.var_name()).format("+"))?;
        }
    }

    let mut output_exprs = Vec::new();
    let max_width = program
        .reactions
        .iter()
        .map(|reaction| {
            let reaction_name = reaction.var_name();
            let pretty_name = reaction.label.as_deref().unwrap_or(&reaction_name);
            pretty_name.chars().count()
        })
        .max()
        .unwrap_or(0);

    for reaction in &program.reactions {
        let reaction_name = reaction.var_name();
        let pretty_name = reaction.label.as_deref().unwrap_or(&reaction_name);
        output_exprs.push(format!("if fix({reaction_name}) > 0 then \"{pretty_name:<width$} =\" ++ show_float(8, 5, {reaction_name}) ++ \"\\n\" else \"\" endif", width=max_width))
    }

    writeln!(w, "output [{}];", output_exprs.join(",\n"))?;

    Ok(())
}

fn expected_str<'a>(word: &str, expected: impl IntoIterator<Item=impl AsRef<str> + 'a>) -> String {
    let expected = expected.into_iter().collect_vec();
    let expected = expected.iter().map(|i| i.as_ref()).collect_vec();

    if expected.len() == 0 {
        "".to_string()
    } else if expected.len() == 1 {
        format!("{word}{}", expected[0])
    } else {
        let (last, rest) = expected.split_last().unwrap();
        format!("{word}{} or {last}", rest.join(","))
    }
}

fn parse<'s>(input: &'s str, filename: &str) -> Program<'s> {
    let program = match grammar::ProgramParser::new().parse(&input) {
        Ok(i) => { i }
        Err(e) => {
            let report = match e {
                ParseError::InvalidToken { location } => {
                    Report::build(ReportKind::Error)
                        .with_message("invalid token")
                        .with_label(Label::new(location..location+1).with_message("here"))
                        .finish()
                }
                ParseError::UnrecognizedEof { location, expected } => {
                    Report::build(ReportKind::Error)
                        .with_message("unexpected end of file")
                        .with_label(Label::new(location..location + 1).with_message(expected_str("expected ", &expected)))
                        .finish()
                }
                ParseError::UnrecognizedToken { token: (from, tok, to), expected } => {
                    Report::build(ReportKind::Error)
                        .with_message(format!("invalid token '{tok}'"))
                        .with_label(Label::new(from..to).with_message(expected_str("expected ", &expected)))
                        .finish()
                }
                ParseError::ExtraToken { token: (from, tok, to) } => {
                    Report::build(ReportKind::Error)
                        .with_message(format!("unexpected token '{tok}'"))
                        .with_label(Label::new(from..to).with_message("no rule expects this token"))
                        .finish()
                }
                ParseError::User { error: (from, err, to) } => {
                    Report::build(ReportKind::Error)
                        .with_message("parse error".to_string())
                        .with_label(Label::new(from..to).with_message(err))
                        .finish()
                }
            };

            exit_report(
                &report,
                Source::from(input.to_string())
                    .with_filename(filename)
            );
        }
    };
    program
}

