# Streamson Bin

Splits stdout (in JSON format) according to given options.
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
```
cat input.json | sson extract --depth 2
```

Output:
```
{"name": "user1", "authors": ["admins", "staff"], "password": "secret1"}{"name": "user2", "authors": ["staff"], "password": "secret2"}
```

### Mask password
```
cat input.json | \
	sson extract --depth 2 | \
	sson convert --replace '"***"' --simple '{"password"}'
```

Output:
```
{"name": "user1", "groups": ["admins", "staff"], "password": "***"}{"name": "user2", "groups": ["staff"], "password": "***"}
```

### Remove groups
```
cat input.json | \
	sson extract --depth 2 | \
	sson convert --replace '"***"' --simple '{"password"}' | \
	sson filter --simple '{"groups"}'
```

Output:
```
{"name": "user1", "password": "***"}{"name": "user2", "password": "***"}
```


### Store names of users into a separate files
```
cat input.json | \
	sson extract --depth 2 | \
	sson convert --replace '"***"' --simple '{"password"}' | \
	sson filter --simple '{"groups"}' | \
	sson trigger --file simple '{"name"}' names.out
```

Output:
```
{"name": "user1", "password": "***"}{"name": "user2", "password": "***"}
```

names.out:
```
"user1"
"user2"
```
