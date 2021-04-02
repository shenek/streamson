# Streamson Bin (sson)

Processes stdout (in JSON format) according to given options.
It is supposed to be memory efficient and fast to process.


## Examples
Consider following context of `input.json` file:
```json
{
	"users": [
		{"name": "user1", "groups": ["admins", "staff"], "password": "secret1"},
		{"name": "user2", "groups": ["staff"], "password": "secret2"}
	]
}

```

### Split each user into separate JSON
```bash
cat input.json | sson extract -m depth:2
```

Output:
```json
{"name": "user1", "authors": ["admins", "staff"], "password": "secret1"}{"name": "user2", "authors": ["staff"], "password": "secret2"}
```

### Mask password
```bash
cat input.json | \
	sson extract -m depth:2 | \
	sson convert -m 'simple:{"password"}' -h replace:'"***"'
```

Output:
```json
{"name": "user1", "groups": ["admins", "staff"], "password": "***"}{"name": "user2", "groups": ["staff"], "password": "***"}
```

### Remove groups
```bash
cat input.json | \
	sson extract -m depth:2 | \
	sson convert -m 'simple:{"password"}' -h replace:'"***"' | \
	sson filter -m 'simple:{"groups"}'
```

Output:
```json
{"name": "user1", "password": "***"}{"name": "user2", "password": "***"}
```

### Make JSON more readable
```bash
cat input.json | \
	sson extract -m depth:2 | \
	sson convert -m 'simple:{"password"}' -h replace:'"***"' | \
	sson filter -m 'simple:{"groups"}' | \
	sson all -h indenter:2
```

Output:
```json
{
  "name": "user1",
  "password": "***"
}
{
  "name": "user2",
  "password": "***"
}
```


### Store names of the users into a separate files
```bash
cat input.json | \
	sson extract -m depth:2 | \
	sson convert -m 'simple:{"password"}' -h replace:'"***"' | \
	sson filter -m 'simple:{"groups"}' | \
	sson all -h indenter:2 | \
	sson trigger -m 'simple:{"name"}' -h file:names.out
```

Output:
```json
{
  "name": "user1",
  "password": "***"
}
{
  "name": "user2",
  "password": "***"
}
```

names.out:
```
"user1"
"user2"
```

### Remove quotes from the output file
```bash
cat input.json | \
	sson extract -m depth:2 | \
	sson convert -m 'simple:{"password"}' -h replace:'"***"' | \
	sson filter -m 'simple:{"groups"}' | \
	sson all -h indenter:2 | \
	sson trigger -m 'simple:{"name"}' -h unstringify -h file:names.out
```

Output:
```json
{
  "name": "user1",
  "password": "***"
}
{
  "name": "user2",
  "password": "***"
}
```

names.out:
```
user1
user2
```
