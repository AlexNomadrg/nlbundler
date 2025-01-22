use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::Parser;
use regex::Regex;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the project to be bundled
    #[arg(short, long)]
    project_path: PathBuf,
    /// (Default: main.lua) main lua file for other packages to be included in
    #[arg(short, long)]
    main_file: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let project_dir = args.project_path;
    let main_file = args.main_file.unwrap_or("main.lua".to_owned());
    // find main file
    let Ok(mut mainfile) = fs::File::open(project_dir.join(&main_file)) else {
        return Result::Err(AppError::MainFileNotFound(main_file).into());
    };
    let mut maincontents = String::new();
    mainfile.read_to_string(&mut maincontents)?;

    // println!("{}", maincontents);

    let include_regex_string = r"--#include\(\w+\)";
    let includes_regex = Regex::new(&include_regex_string)?;
    let includes: Vec<_> = includes_regex.find_iter(&maincontents).collect();

    let packages: Vec<_> = includes
        .iter()
        .map(|&m| {
            let package_name = m
                .as_str()
                .replace(r#"--#include("#, "")
                .replace(r#")"#, "")
                .replace(r#"""#, "");
            Package::new(
                package_name.clone(),
                m.clone(),
                find_package(&package_name, &project_dir),
            )
        })
        .collect();
    let mut edited_main = maincontents.clone();
    for package in packages {
        let Some(path) = package.path else {
            return Result::Err(PackageError::NotFound(package.name).into());
        };
        let mut package_file = fs::File::open(path)?;
        let mut package_contens = String::new();
        package_file.read_to_string(&mut package_contens)?;
        edited_main = edited_main.replace(package.matched.as_str(), &package_contens);
    }

    // let strs: Vec<_> = includes.iter().map(|m| m.as_str()).collect();
    // println!("{:#?}", packages);
    // println!("{:?}", strs);
    print!("{edited_main}");
    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum PackageError {
    #[error("Package ({0}) is nowhere to be found!")]
    NotFound(String),
}

#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("({0}) is nowhere to be found!")]
    MainFileNotFound(String),
}

#[derive(Debug)]
struct Package<'a> {
    pub name: String,
    pub matched: regex::Match<'a>,
    pub path: Option<PathBuf>,
}

impl<'a> Package<'a> {
    pub fn new(name: String, matched: regex::Match<'a>, path: Option<PathBuf>) -> Self {
        Self {
            name,
            matched,
            path,
        }
    }
}

fn find_package(pkg_name: &str, dir: &Path) -> Option<PathBuf> {
    let pkg_name = if pkg_name.ends_with(".lua") {
        pkg_name.to_owned()
    } else {
        format!("{pkg_name}{}", r".lua")
    };
    let dir_recursive = walkdir::WalkDir::new(dir);
    let mut package_candidates = dir_recursive
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| -> bool {
            let Some(file_name) = e.file_name().to_str() else {
                return false;
            };
            return file_name == pkg_name;
        });
    // FIX: Solve ambiguity in found package files
    package_candidates.next().map(|e| e.path().to_owned())
}
