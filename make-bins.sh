#!/bin/bash
for width in {2..15}; do
    for height in $(seq 2 $width); do
        SQUARE=$([ $width = $height ] && echo ",square")
        echo "building $width x $height $SQUARE"
        cargo build --release --no-default-features --features="width-$width,height-$height,$SQUARE"
        cp target/release/rust-word-square bin/rws-${width}x${height}
    done
done