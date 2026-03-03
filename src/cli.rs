use std::path::PathBuf;

use clap::Parser;
use colored::*;

use crate::consts::DEFAULT_EXTENSIONS;

//
// #[command(author, version, about = "Wallpaper renamer — name-[WxH].ext")]

#[rustfmt::skip]
#[derive(Parser, Debug)]
#[command(
    name = crate::crate_name!(),
    author = crate::crate_authors!(),
    version = crate::crate_version!(),
    about = crate::crate_description!(),
    long_about = "\nIt's for [M]ass [R]e-[n]aming, images mostly.\n
Organizes your wallpapers by renaming them to the format 'name-[WxH].ext',
where W and H are the width and height of the image, respectively.
It can also move the renamed files to a specified output directory.
",
    arg_required_else_help = true,
    // Allows for the custom parsing of the version flag
    disable_version_flag = true,
    styles = get_styles()
)]
pub struct Cli {
    #[arg(index = 1, default_value = ".", help = "The input directory to scan for wallpapers. Defaults to the current directory.", value_hint = clap::ValueHint::DirPath)]
    pub input: PathBuf,

    #[arg(short = 'E', long = "extensions", value_delimiter = ',', default_values = DEFAULT_EXTENSIONS)]
    pub exts: Vec<String>,

    #[arg(short = 'L', long = "follow_links", help = "Follow symbolic links when scanning for wallpapers. By default, symbolic links are not followed to prevent potential infinite loops.", value_hint = clap::ValueHint::CommandString)]
    pub follow_links: bool,

    #[arg(short = 'd', long = "dry_run", help = "Perform a dry run, showing the actions that would be taken without making any changes to the filesystem.", value_hint = clap::ValueHint::CommandString)]
    pub dry_run: bool,

    #[arg(short = 'm', long = "move", help = "Move renamed wallpapers to the output directory instead of copying them. This flag is ignored if the output directory is not specified.", value_hint = clap::ValueHint::CommandString)]
    pub r#move: bool,

    #[arg(short = 'o', long = "output", help = "The output directory to move renamed wallpapers to. If not specified, wallpapers will be renamed in place.", value_hint = clap::ValueHint::DirPath)]
    pub output: Option<PathBuf>,

    /// Other version flag
    #[arg(short = 'v', short_alias = 'V', long = "version", help = "Prints version information")]
    pub version: bool,
}

use crate::Action;

impl Cli {
    pub fn new() -> Self {
        let s = Self::parse();
        if s.version {
            println!("{} {}", crate::crate_name!(), crate::crate_version!());
            std::process::exit(0);
        }

        if s.input.as_os_str().is_empty() || !s.input.exists() {
            eprintln!("{} Input directory cannot be empty or non-existent", "Error:".red().bold());
            std::process::exit(1);
        }

        if s.dry_run {
            println!("Performing a dry run. No changes will be made to the filesystem.");
        }

        s
    }

    pub fn dry_run(&self, actions: &Vec<Action>) {
        for action in actions {
            println!("{}", format!("{}", action).blue().bold());
        }
    }
}

pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))), // When a command is inc. This is the tag collor for 'Usage:'
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Blue))), // Main headers in the help menu (e.g. Arguments, Options)
        )
        .literal(
            anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightWhite))), // Strings for args etc { -t, --total }
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)))
                .effects(anstyle::Effects::ITALIC),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}
