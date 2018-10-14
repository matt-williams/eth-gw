#!/bin/bash
HEX=$(echo $(xxd -p -g0) | sed -e 's/ //g')
ADDR=$(echo $HEX | cut -c 1-40) 
CONTENT=$(echo $HEX | cut -c 41-)0000000000
export ENS_ADDR=adb9e045ff13e72662d541eb334c59f4634ef8b0
~/rust-ens/target/debug/examples/set_address $1 $ADDR
~/rust-ens/target/debug/examples/set_content $1 $CONTENT
