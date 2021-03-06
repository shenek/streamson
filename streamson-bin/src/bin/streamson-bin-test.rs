use assert_cmd::Command;
use predicates::prelude::*;
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
        .arg("depth:2")
        .arg("-m")
        .arg(r#"simple:{"logs"}"#)
        .arg("-m")
        .arg(r#"regex:^\{"groups"\}"#)
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
        .arg("depth:2")
        .arg("-m")
        .arg(r#"simple:{"logs"}"#)
        .arg("-m")
        .arg(r#"regex:^\{"users"\}"#)
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
        .arg("depth:2")
        .arg("-m")
        .arg(r#"regex:^\{"logs"\}"#)
        .arg("-m")
        .arg(r#"simple:{"users"}"#)
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
        .arg("depth:2")
        .arg("-m")
        .arg(r#"regex:^\{"logs"\}"#)
        .arg("-m")
        .arg(r#"simple:{"users"}"#)
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
        .arg(r#"simple:{"users"}[]{"name"}"#)
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
        .arg(r#"simple:{"logs"}[]"#)
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
        .arg(r#"simple:{"users"}[]{"name"}"#)
        .arg("-h")
        .arg("regex:s/([a-z]+)/USER_$1/")
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
        .arg(r#"simple.1:{"logs"}[]"#)
        .arg("-h")
        .arg("file.1:/dev/stderr")
        .arg("-m")
        .arg(r#"regex.2:^\{"users"\}$"#)
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
    print!("ALL ANALYSER ");
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

    print!("ALL INDENTER ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("all")
        .arg("-h")
        .arg("indenter:2")
        .write_stdin(INPUT_DATA)
        .assert()
        .success()
        .stdout(
            r#"{
  "users": [
    {
      "name": "carl",
      "id": 1
    },
    {
      "name": "paul",
      "id": 2
    }
  ],
  "groups": [
    {
      "name": "admin",
      "gid": 1
    },
    {
      "name": "staff",
      "gid": 2
    }
  ],
  "logs": [
    "null",
    "{}",
    "[]"
  ]
}
"#,
        );
    println!("OK");

    print!("ALL SHORTEN ");
    Command::new(cmd_str)
        .arg("-b")
        .arg("10")
        .arg("all")
        .arg("-h")
        .arg("shorten:5")
        .write_stdin(INPUT_DATA)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "handler `shorten` can not be used in `all` strategy.",
        ));
    println!("OK (failed)");
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
