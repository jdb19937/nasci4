# nasci4
learning rust by writing this key-value store using time-decaying hashcash to establish consensus

## About
	This is a key-value store with a web interface that demonstrates
	eventual consistency and storage-rental using time-decaying
	hashcash.  Key-value pairs are stored in a hashtree that is
	used to synchronize with peers.  Each node periodically broadcasts
	the root hash (comprising the entire set), and when encountering
	anything that differs from its own root hash, responds with a
	request to expand that hash.  When receiving such a request,
	a node responds with either the hashes of the two children
	of that node (which if unknown can trigger subsequent requests),
	or if the hash corresponds to a single key-value, that key-value pair
	itself.  With this method, if there are n keys and k of them are
	different between two nodes it should take about k*log2(n) interactions
	for them to synchronize.

	Each key-value pair has an associated proof-of-work and timestamp.
	With a constant decay rate specified, there can be computed 
	a priority value used to resolve conflicts,

	priority = exp(-decay * age) / hash

	where decay is a constant frequency, age is the time elapsed since
	the timestamp, and hash is an integer value of the hash function
	(that the proof-of-work creator attempts to minimize).  The higher
	the "priority" value, the higher the priority of the update.  This
	is a consistent ordering across nodes, no matter what is the time
	skew, because two priorities can be compared using absolute
	timestamps without reference to the system time.

	The system time is used to ensure that only updates with a
	timestamp in the past are processed (so if there is a time skew,
	updates can appear on the future-skewed nodes first, but conflicts
	resolve as soon as ages are positive).  So a conflict can be
	present only for as long a duration as the time skew.

	The idea is that you can "rent" a key for some expected amount of
	time by creating a proof-of-work, because it would require an
	even harder proof-of-work to immediately override it.
	With time passing, a key becomes easier to override because
	its value is decaying.

## Build
	cargo build

## Run
	./start.sh  # launches 3 instances on ports 5555, 5556, 5557
	./fill.sh   # fills instance on 5555 with 50 key-value pairs
	./stop.sh   # kill all the instances


## Demo
	Run ./start.sh, and then ./fill.sh, and point your browser at
	http://localhost:5555/.

	To retrieve the value for a key, enter the key (an integer),
	and click "get".  fill.sh already inserted keys 5 through 54,
	with each value equal to the key.  So if you "get" with key=43,
	it should return with val=43.  fill.sh performed the updates on
	the server on port 5555, but each update should replicate to the
	other two servers soon after fill.sh is run.

	To assign the value of a key, enter the key and the value
	(both integers), and click "set".  The server will
	compute a proof of work with at least the minlogwork value
	specified.  For the update to be accepted, this must exceed
	the old value for the key.  It will be very slow if it is > 15.

	After setting a key, click the links at the top to the servers
	running on different ports to check if replication is working.
	There is a 10-second update heartbeat and a log at the bottom
	of each webpage.
