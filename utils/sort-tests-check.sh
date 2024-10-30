#!/bin/sh

tmpdir=$(mktemp -d)
filename="$1"
linerange="$2"

if [ "$filename" = "" ] || [ "$filename" = "--help" ]; then
    echo "Usage: $0 FILENAME [FROMLINE,TOLINE]"
    echo
    echo "  e.g: $0 src/tests.rs 20,50"
    exit
fi

if [ ! "$linerange" = "" ]; then
    text=$(cat "$filename" | sed -n "${linerange}p")
else
    text=$(cat "$filename")
fi

echo "$text" | grep -Pzo '#\[test\]\nfn test.+\n' | grep -Poa 'fn test.+' > "${tmpdir}/a.txt"
cat "${tmpdir}/a.txt" | sort > "${tmpdir}/b.txt"
diff --color=always -u "${tmpdir}/a.txt" "${tmpdir}/b.txt"

rm -rf "$tmpdir"
