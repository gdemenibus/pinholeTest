#!/usr/bin/env fish

#set sep (perf stat ./target/release/light_field_test -h -t sep)
#set stereo (perf stat ./target/release/light_field_test -h -t stereo)
#set load (perf stat ./target/release/light_field_test -h -t load)

echo "SEP FLOPS"
likwid-perfctr -f -g FLOPS_SP -C 0-10 ./target/release/light_field_test -h -t sep
sleep 1

echo "STEREO FLOPS"
likwid-perfctr -g FLOPS_SP -C 0-10 ./target/release/light_field_test -h -t stereo
sleep 1

echo "LOAD FLOPS"
likwid-perfctr -g FLOPS_SP -C 0-10 ./target/release/light_field_test -h -t load
sleep 1


echo "SEP CACHE"
likwid-perfctr -g CACHE -C 0-10 ./target/release/light_field_test -h -t sep
sleep 1

echo "STEREO CACHE"
likwid-perfctr -g CACHE -C 0-10 ./target/release/light_field_test -h -t stereo
sleep 1
echo "LOAD CACHE"
likwid-perfctr -g CACHE -C 0-10 ./target/release/light_field_test -h -t load
sleep 1

