#!/bin/bash
for width in {2..15}; do
    for height in $(seq 2 $width); do
        bin/rws-${width}x${height} compute -q --ignore-empty-wordlist "$@"
    done
done