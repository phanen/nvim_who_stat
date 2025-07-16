use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::process::Command;

#[derive(Serialize, Debug)]
struct Person {
    name: String,
    repos: HashMap<String, u32>,
    repo_count: u32,
    commit_count: u32,
}

#[derive(Serialize)]
struct Repo {
    name: String,
    contributors: usize,
}

#[derive(Serialize)]
struct RepoDate {
    name: String,
    date: String,
    time: i64,
}

fn get_plugin_dirs(base_dir: &str, ignore: &HashSet<&str>) -> Vec<(String, String)> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(base_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if ignore.contains(name.as_str()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() && path.join(".git").is_dir() {
            dirs.push((name, path.to_string_lossy().to_string()));
        }
    }
    dirs
}

fn main() {
    let alias = HashMap::from([("phanen", "phanium")]);
    let ignore: HashSet<&str> = [
        "lazy.nvim",
        "fzf",
        "lazygit",
        "fish-shell",
        "kitty",
        "inline",
        "nushell",
        "atuin",
        "zignvim",
    ]
    .iter()
    .cloned()
    .collect();

    let plugin_base = format!("{}/lazy", std::env::var("HOME").unwrap());
    let plugins = get_plugin_dirs(&plugin_base, &ignore);

    let mut people: HashMap<String, Person> = HashMap::new();
    let mut repos: Vec<Repo> = Vec::new();

    for (plugin, dir) in &plugins {
        // Bullshit: https://stackoverflow.com/questions/72804787/git-produces-no-output-when-called-from-script-via-cron-log-shortlog
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "git -C '{}' log --pretty=short | GIT_PAGER= PAGER= git shortlog -sn",
                dir
            ))
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut contributors = 0;
        for line in stdout.lines() {
            if let Some((commits, author)) = line.trim().split_once('\t') {
                let commits: u32 = commits.parse().unwrap_or(0);
                let author = author.trim();
                let name = alias.get(author).unwrap_or(&author).to_string();
                let person = people.entry(name.clone()).or_insert(Person {
                    name: name.clone(),
                    repos: HashMap::new(),
                    repo_count: 0,
                    commit_count: 0,
                });
                person.repos.insert(plugin.clone(), commits);
                person.repo_count += 1;
                person.commit_count += commits;
                contributors += 1;
            }
        }
        repos.push(Repo {
            name: plugin.clone(),
            contributors,
        });
    }

    let persons: Vec<_> = people.values().collect();
    let who_json = serde_json::to_string_pretty(&persons).unwrap();
    fs::write("/tmp/tmp/who.json", who_json).unwrap();

    repos.sort_by(|a, b| b.contributors.cmp(&a.contributors));
    let repos_json = serde_json::to_string_pretty(&repos).unwrap();
    fs::write("/tmp/tmp/repos.json", repos_json).unwrap();

    let mut repo_dates = Vec::new();
    for (plugin, dir) in &plugins {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "git -C '{}' log --reverse --format=\"format:%ad\" | head -1",
                dir
            ))
            .output()
            .unwrap();
        let date = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // e.g. "Fri Jan 31 10:39:15 2014 -0300"
        let time = chrono::DateTime::parse_from_str(&date, "%a %b %d %H:%M:%S %Y %z")
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        repo_dates.push(RepoDate {
            name: plugin.to_string(),
            date,
            time,
        });
    }
    repo_dates.sort_by_key(|r| r.time);
    let repo_dates_json = serde_json::to_string_pretty(&repo_dates).unwrap();
    fs::write("/tmp/tmp/who-date.json", repo_dates_json).unwrap();
}
