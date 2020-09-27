pub fn usize_validator(input: String) -> Result<(), String> {
    let res = input.parse::<usize>().map_err(|err| err.to_string())?;
    if res == 0 {
        Err("Buffer can't have 0 size".into())
    } else {
        Ok(())
    }
}
