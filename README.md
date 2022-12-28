# nasci4
learning rust by writing this key-value store using time-decaying hashcash to establish consensus


Build:
	cargo build

Run:
	./start.sh  # launches 3 instances on ports 5555, 5556, 5557
        ./fill.sh   # fills instance on 5555 with 50 key-value pairs
	./stop.sh   # kill all the instances

	point browser at http://localhost:5555/
	to retrieve the value for a key, click "get"
	to assign the value of a key, click "set"
