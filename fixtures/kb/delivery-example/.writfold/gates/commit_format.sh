#!/bin/sh
# Check a commit message draft: a subject of 1 to 72 characters, a blank
# second line, and body lines of at most 72 characters. Each finding is
# printed as "<line>: <text>"; the exit status is 1 when there is any.
file=$1
n=0
found=0
while IFS= read -r line || [ -n "$line" ]; do
  n=$((n + 1))
  len=${#line}
  if [ "$n" -eq 1 ] && [ "$len" -eq 0 ]; then
    echo "1: the subject line is empty"; found=1
  elif [ "$n" -eq 1 ] && [ "$len" -gt 72 ]; then
    echo "1: subject has $len characters; at most 72"; found=1
  elif [ "$n" -eq 2 ] && [ "$len" -ne 0 ]; then
    echo "2: the second line must be blank"; found=1
  elif [ "$n" -gt 2 ] && [ "$len" -gt 72 ]; then
    echo "$n: body line has $len characters; wrap at 72"; found=1
  fi
done < "$file"
exit $found
