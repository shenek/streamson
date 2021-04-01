pub fn usize_validator(input: &str) -> Result<(), String> {
    let res = input.parse::<usize>().map_err(|err| err.to_string())?;
    if res == 0 {
        Err("Buffer can't have 0 size".into())
    } else {
        Ok(())
    }
}

/// Split arguments to get Name, Group and Definition
pub fn split_argument<S>(value: S) -> (String, String, Vec<String>, String)
where
    S: ToString,
{
    let splitted = value
        .to_string()
        .splitn(2, ':')
        .map(String::from)
        .collect::<Vec<String>>();

    let (name_group_options, definition) = match splitted.len() {
        1 => (splitted[0].clone(), String::default()),
        2 => (splitted[0].clone(), splitted[1].clone()),
        _ => unreachable!(),
    };

    let splitted2 = name_group_options
        .splitn(2, '.')
        .map(String::from)
        .collect::<Vec<String>>();

    let (name, group_options) = match splitted2.len() {
        1 => (splitted2[0].clone(), String::default()),
        2 => (splitted2[0].clone(), splitted2[1].clone()),
        _ => unreachable!(),
    };

    let mut splitted3 = group_options
        .split(',')
        .map(String::from)
        .collect::<Vec<String>>();

    let group = splitted3.remove(0);
    let options = splitted3.to_vec();

    (name, group, options, definition)
}
