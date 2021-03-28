pub fn usize_validator(input: &str) -> Result<(), String> {
    let res = input.parse::<usize>().map_err(|err| err.to_string())?;
    if res == 0 {
        Err("Buffer can't have 0 size".into())
    } else {
        Ok(())
    }
}

/// Split arguments to get Name, Group and Definition
pub fn split_argument<S>(value: S) -> (String, String, String)
where
    S: ToString,
{
    let splitted = value
        .to_string()
        .splitn(2, ':')
        .map(String::from)
        .collect::<Vec<String>>();

    let (name_and_group, definition) = match splitted.len() {
        1 => (splitted[0].clone(), String::default()),
        2 => (splitted[0].clone(), splitted[1].clone()),
        _ => unreachable!(),
    };

    let splitted2 = name_and_group
        .splitn(2, '.')
        .map(String::from)
        .collect::<Vec<String>>();

    let (name, group) = match splitted2.len() {
        1 => (splitted2[0].clone(), String::default()),
        2 => (splitted2[0].clone(), splitted2[1].clone()),
        _ => unreachable!(),
    };

    (name, group, definition)
}
