#!/bin/bash
target/debug/nasci4 0:5555 > log.0:5555.txt & target/debug/nasci4 0:5556 > log.0:5556.txt & target/debug/nasci4 0:5557 > log.0:5557.txt &
