#!/bin/bash
set -e
set -u

PROMPT="❯"

enter() {
	INPUT=$1
	DELAY=1

	prompt
	sleep "$DELAY"
	type "$INPUT"
	sleep 0.5
	printf '%b' "\\n"
	eval "$INPUT" || true
}

prompt() {
	printf '%b ' $PROMPT
}

type() {
	printf '%b' "$1"
}

main() {
    IFS='%'

	enter "# Consider we have a large compressed JSON file."
	enter "du -h /tmp/rpss.json"
	enter "# Our memory is limited to 40000KB."
	enter "ulimit -v 40000"
	enter "# Lets try to reformat to more readable representation in python."
	enter "cat /tmp/rpss.json | python3 -m json.tool > /tmp/rpss-nice.json"
	enter "# It crashed because the JSON couldn't fit into memory."
	enter "# Lets try to do the same using streamson."
	enter "cat /tmp/rpss.json | sson all -h indenter:4 > /tmp/rpss-nice.json"
	enter "# It seems to pass. Lets check the output."
	enter "head /tmp/rpss-nice.json"
	enter "# Good. Output JSON seems to be properly reformatted."

	prompt

	sleep 3

	echo ""

	unset IFS
}

main
