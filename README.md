# nasci4
learning rust by writing this key-value store using time-decaying hashcash to establish consensus


Build:
	cargo build

Run:
	target/debug/nasci4 0:5555 &
	target/debug/nasci4 0:5556 &
	target/debug/nasci4 0:5557 &

	point browser at http://localhost:5555/
	to retrieve the value for a key, click "get"
	to assign the value of a key, click "set"
