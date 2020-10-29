use assert_cmd::Command;
use std::env;

const INPUT_DATA: &str = r#"{
    "users": [{"name": "carl", "id": 1}, {"name": "paul", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": ["aaa", "bbb", "ccc"]
}"#;

fn filter(cmd_str: &str) {
    print!("FILTER ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("filter")
        .arg("--depth")
        .arg("2")
        .arg("--simple")
        .arg(r#"{"logs"}"#)
        .arg("--simple")
        .arg(r#"{"groups"}"#)
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
    "users": []
}"#,
        );
    println!("OK");
}

fn extract(cmd_str: &str) {
    print!("EXTRACT ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("extract")
        .arg("--depth")
        .arg("2")
        .arg("--simple")
        .arg(r#"{"logs"}"#)
        .arg("--simple")
        .arg(r#"{"users"}"#)
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"[{"name": "carl", "id": 1}, {"name": "paul", "id": 2}]{"name": "admin", "gid": 1}{"name": "staff", "gid": 2}["aaa", "bbb", "ccc"]"#,
        );
    println!("OK");
}

fn convert(cmd_str: &str) {
    print!("CONVERT REPLACE ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("convert")
        .arg("--depth")
        .arg("2")
        .arg("--simple")
        .arg(r#"{"logs"}"#)
        .arg("--simple")
        .arg(r#"{"users"}"#)
        .arg("--replace")
        .arg(r#""...""#)
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
    "users": "...",
    "groups": ["...", "..."],
    "logs": "..."
}"#,
        );
    println!("OK");

    print!("CONVERT SHORTEN ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("convert")
        .arg("--simple")
        .arg(r#"{"users"}[]{"name"}"#)
        .arg("--shorten")
        .arg(r#"1"#)
        .arg(r#"..""#)
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
    "users": [{"name": "c..", "id": 1}, {"name": "p..", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": ["aaa", "bbb", "ccc"]
}"#,
        );
    println!("OK");
}

fn trigger(cmd_str: &str) {
    print!("TRIGGER ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("trigger")
        .arg("--file")
        .arg("simple")
        .arg(r#"{"logs"}[]"#)
        .arg("/dev/stdout")
        .arg("--print")
        .arg("simple")
        .arg(r#"{"users"}"#)
        .arg("--print-with-header")
        .arg("depth")
        .arg("2")
        .arg("-s")
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{"users"}[0]: {"name": "carl", "id": 1}
{"users"}[1]: {"name": "paul", "id": 2}
[{"name": "carl", "id": 1}, {"name": "paul", "id": 2}]
{"groups"}[0]: {"name": "admin", "gid": 1}
{"groups"}[1]: {"name": "staff", "gid": 2}
{"logs"}[0]: "aaa"
"aaa"
{"logs"}[1]: "bbb"
"bbb"
{"logs"}[2]: "ccc"
"ccc"
JSON structure:
  <root>: 1
  {"groups"}: 1
  {"groups"}[]: 2
  {"groups"}[]{"gid"}: 2
  {"groups"}[]{"name"}: 2
  {"logs"}: 1
  {"logs"}[]: 3
  {"users"}: 1
  {"users"}[]: 2
  {"users"}[]{"id"}: 2
  {"users"}[]{"name"}: 2
"#,
        );
    println!("OK");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 2);
    filter(&args[1]);
    extract(&args[1]);
    convert(&args[1]);
    trigger(&args[1]);
}
