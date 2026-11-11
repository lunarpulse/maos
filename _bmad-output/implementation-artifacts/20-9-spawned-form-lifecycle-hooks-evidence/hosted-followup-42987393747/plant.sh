#!/bin/bash
# usage: plant.sh name file old new -- cargo-args...
name=$1; file=$2; old=$3; new=$4; shift 5
cp "$file" /tmp/dev20hosted/plants/$name.orig
python3 - "$file" "$old" "$new" <<'PY'
import sys
f,o,n=sys.argv[1:4]
s=open(f).read(); assert s.count(o)==1,(f,o,s.count(o)); open(f,'w').write(s.replace(o,n))
PY
cargo test "$@" > /tmp/dev20hosted/plants/$name.log 2>&1; echo "rc=$?" >> /tmp/dev20hosted/plants/$name.log
cp /tmp/dev20hosted/plants/$name.orig "$file"; cmp /tmp/dev20hosted/plants/$name.orig "$file" && echo "$name restored-cmp-ok"; touch "$file"
grep -n "^test result\|^rc=" /tmp/dev20hosted/plants/$name.log
