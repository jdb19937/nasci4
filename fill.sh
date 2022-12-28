#!/bin/sh
for i in `seq 5 70`; do
  curl -v "http://localhost:5555/?k=$i&v=$i&op=set"
done
