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
        .arg("-m")
        .arg("depth")
        .arg("2")
        .arg("-m")
        .arg("simple")
        .arg(r#"{"logs"}"#)
        .arg("-m")
        .arg("regex")
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
        .arg("-m")
        .arg("depth")
        .arg("2")
        .arg("-m")
        .arg("simple")
        .arg(r#"{"logs"}"#)
        .arg("-m")
        .arg("regex")
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
        .arg("-m")
        .arg("depth")
        .arg("2")
        .arg("-m")
        .arg("regex")
        .arg(r#"^\{"logs"\}"#)
        .arg("-m")
        .arg("simple")
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
        .arg("-m")
        .arg("depth")
        .arg("2")
        .arg("-m")
        .arg("regex")
        .arg(r#"^\{"logs"\}"#)
        .arg("-m")
        .arg("simple")
        .arg(r#"{"users"}"#)
        .arg("-h")
        .arg(r#"replace:"...""#)
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
        .arg("-m")
        .arg("simple")
        .arg(r#"{"users"}[]{"name"}"#)
        .arg("-h")
        .arg(r#"shorten:1,..""#)
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
        .arg("-m")
        .arg("simple")
        .arg(r#"{"logs"}[]"#)
        .arg("-h")
        .arg("unstringify")
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
        .arg("-m")
        .arg("simple")
        .arg(r#"{"users"}[]{"name"}"#)
        .arg("-h")
        .arg("regex:([a-z]+),USER_$1")
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
        .arg("-m")
        .arg("simple:1")
        .arg(r#"{"logs"}[]"#)
        .arg("-h")
        .arg("file.1:/dev/stderr")
        .arg("-m")
        .arg("regex:2")
        .arg(r#"^\{"users"\}$"#)
        .arg("-h")
        .arg("file.2:/dev/stderr")
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
    "users": [{"name": "carl", "id": 1}, {"name": "paul", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": ["null", "{}", "[]"]
}"#,
        )
        .stderr(
            r#"[{"name": "carl", "id": 1}, {"name": "paul", "id": 2}]
"null"
"{}"
"[]"
"#,
        );

    println!("OK");
}

fn all(cmd_str: &str) {
    print!("ALL ANALYSER");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("all")
        .arg("-h")
        .arg("analyser")
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stderr(
            r#"JSON structure:
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
        )
        .stdout(
            r#"{
    "users": [{"name": "carl", "id": 1}, {"name": "paul", "id": 2}],
    "groups": [{"name": "admin", "gid": 1}, {"name": "staff", "gid": 2}],
    "logs": ["null", "{}", "[]"]
}"#,
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
    all(&args[1]);
}
