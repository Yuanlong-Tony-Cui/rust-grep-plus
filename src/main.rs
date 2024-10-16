use colored::*;
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::process;
use walkdir::WalkDir;
use regex::Regex;

/*
    A `Config` struct saves the parsed result.
*/
#[derive(Debug)]
struct Config {
    query_str: String,
    target_files: Vec<String>,
    case_insensitive: bool,
    print_line_numbers: bool,
    invert_match: bool,
    recursive_search: bool,
    print_filenames: bool,
    colored_output: bool,
}

impl Config {
    fn new(args: &[String]) -> Result<Config, &str> {
        /*
            The `'static` ensures the lifetime of the string slice
            lasts for the entire duration of the program.
        */
        if args.len() < 3 {
            return Err("not enough arguments");
        }

        /*
        // Print command line arguments:
        for (idx, arg) in args.iter().enumerate() {
            println!("arg[{}]: {}", idx, arg);
        }
        */

        let mut config = Config {
            query_str: args[1].clone(),
            target_files: Vec::new(),
            case_insensitive: false,
            print_line_numbers: false,
            invert_match: false,
            recursive_search: false,
            print_filenames: false,
            colored_output: false,
        };

        // Set flags based on `args`:
        for arg in &args[2..] {
            match arg.as_str() {
                "-i" => config.case_insensitive = true,
                "-n" => config.print_line_numbers = true,
                "-v" => config.invert_match = true,
                "-r" => config.recursive_search = true,
                "-f" => config.print_filenames = true,
                "-c" => config.colored_output = true,
                "-h" | "--help" => {
                    print_help_info();
                    process::exit(0); // exits right away
                },
                // Any other arguments will be treated as files / directories:
                _ => config.target_files.push(arg.clone())
            }
        }
        /*
            NOTE: When the user uses wildcard characters in the filename such
            as "*.md", the shell will first expand the "*.md" into individual filenames.
            Therefore, what the Rust program actually receives as its command-line
            arguments is the expanded list of filenames.
        */

        if config.target_files.is_empty() {
            return Err("No files provided.");
        }

        /*
        // Verify that the shell indeed resolves "*.md" for us:
        for (idx, file) in config.target_files.iter().enumerate() {
            println!("config.target_files[{}]: {}", idx, file);
        }
        */

        Ok(config)
    }
}

fn print_help_info() {
    println!("Usage: grep [OPTIONS] <pattern> <files...>");
    println!("Options:");
    println!("-i                Case-insensitive search");
    println!("-n                Print line numbers");
    println!("-v                Invert match (exclude lines that match the pattern)");
    println!("-r                Recursive directory search");
    println!("-f                Print filenames");
    println!("-c                Enable colored output");
    println!("-h, --help        Show help information");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse the input into a `Config` struct:
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        print_help_info();
        process::exit(1);
    });

    // Execute the command based on the `Config` struct:
    if let Err(e) = execute(config) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}

/*
    We have to `Box` the error since we do not know the error type at compile-time
    (i.e. not statically determined).
    https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/boxing_errors.html
*/
fn execute(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let query_str = if config.case_insensitive {
        config.query_str.to_lowercase()
    } else {
        config.query_str.clone()
    };

    // ASSUMPTION: If "-r" is not used, we simply treat it as a file:
    let target_files = if config.recursive_search {
        collect_files_recursively(&config.target_files)?
    } else {
        config.target_files.clone()
    };

    for target_file in target_files {
        search_in_file(&target_file, &query_str, &config)?;
    }

    Ok(())
}

/*
    We use the `WalkDir` crate to do the recursive search.
*/
fn collect_files_recursively(paths: &[String]) -> Result<Vec<String>, io::Error> {
    let mut files = Vec::new();
    for path in paths {
        for entry in WalkDir::new(path).into_iter().filter_map(|result| result.ok()) {
            if entry.file_type().is_file() {
                files.push(entry.path().display().to_string());
            }
        }
    }
    Ok(files)
}

/*
    We have to `Box` the error since we do not know the error type at compile-time
    (i.e. not statically determined).
    https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/boxing_errors.html
*/
fn search_in_file(
    filename: &str, query_str: &str, config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    /*
        We use the `?` operator to match the `Result` with `Ok()` and `Err()`.
        https://doc.rust-lang.org/rust-by-example/std/result/question_mark.html
        If the "-r" flag is not used, we may end up opening a directory.
    */
    let file = fs::File::open(filename)?;
    let reader = io::BufReader::new(file);

    // Look for matches line by line and print as needed:
    for (idx, line) in reader.lines().enumerate() {
        let line_result = line?; // propagates errors

        let matched = if config.case_insensitive {
            line_result.to_lowercase().contains(query_str)
        } else {
            line_result.contains(query_str)
        };

        let should_print = if config.invert_match { !matched } else { matched };
        if should_print {
            print_match(idx, &line_result, filename, config);
        }
    }

    Ok(())
}

fn print_match(line_idx: usize, line: &str, filename: &str, config: &Config) {
    // Highlight the query string in a given line:
    let formatted_line = if config.colored_output {
        if config.case_insensitive {
            // Highlight exact matches and case-insensitive matches:
            let regex_pattern = Regex::new(
                &format!("(?i){}",
                regex::escape(&config.query_str))
            ).unwrap();
            regex_pattern.replace_all(line, |caps: &regex::Captures| {
                caps[0].red().to_string()
            }).to_string() // uses a closure
        } else {
            // Highlight exact matches only:
            line.replace(&config.query_str, &config.query_str.red().to_string())
        }
    } else {
        line.to_string()
    };

    if config.print_filenames && config.print_line_numbers {
        // Print both filename and line number:
        println!("{}: {}: {}", filename, line_idx + 1, formatted_line);
    } else if config.print_filenames {
        // Print filename only:
        println!("{}: {}", filename, formatted_line);
    } else if config.print_line_numbers {
        // Print line number only:
        println!("{}: {}", line_idx + 1, formatted_line);
    } else {
        // Just print the line itself:
        println!("{}", formatted_line);
    }
}
