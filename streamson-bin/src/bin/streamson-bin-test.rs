use assert_cmd::Command;
use std::env;

const INPUT_DATA: &str = r#"{
    "users": [{"name": "carl", "id": 1}, {"name": "paul", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": ["null", "{}", "[]"]
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
        .arg("--regex")
        .arg(r#"^\{"groups"\}"#)
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
        .arg("--regex")
        .arg(r#"^\{"users"\}"#)
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"[{"name": "carl", "id": 1}, {"name": "paul", "id": 2}]{"name": "admin", "gid": 1}{"name": "staff", "gid": 2}["null", "{}", "[]"]"#,
        );
    println!("OK");

    print!("EXTRACT TO JSON ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("extract")
        .arg("--depth")
        .arg("2")
        .arg("--regex")
        .arg(r#"^\{"logs"\}"#)
        .arg("--simple")
        .arg(r#"{"users"}"#)
        .arg("--separator")
        .arg(",\n")
        .arg("--before")
        .arg("[")
        .arg("--after")
        .arg("]")
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"[[{"name": "carl", "id": 1}, {"name": "paul", "id": 2}],
{"name": "admin", "gid": 1},
{"name": "staff", "gid": 2},
["null", "{}", "[]"]]"#,
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
        .arg("--regex")
        .arg(r#"^\{"logs"\}"#)
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
    "logs": ["null", "{}", "[]"]
}"#,
        );
    println!("OK");

    print!("CONVERT UNSTRINGIFY ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("convert")
        .arg("--simple")
        .arg(r#"{"logs"}[]"#)
        .arg("--unstringify")
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
    "users": [{"name": "carl", "id": 1}, {"name": "paul", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": [null, {}, []]
}"#,
        );
    println!("OK");

    print!("CONVERT REGEX ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("convert")
        .arg("--simple")
        .arg(r#"{"users"}[]{"name"}"#)
        .arg("--regex-convert")
        .arg(r#"([a-z]+)"#)
        .arg(r#"USER_$1"#)
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
    "users": [{"name": "USER_carl", "id": 1}, {"name": "USER_paul", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": ["null", "{}", "[]"]
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
        .arg("regex")
        .arg(r#"^\{"users"\}$"#)
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
{"logs"}[0]: "null"
"null"
{"logs"}[1]: "{}"
"{}"
{"logs"}[2]: "[]"
"[]"
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
