#!/bin/bash
set -e
if [[ "$1" == "" ]]; then
    echo "Usage: $0 <name>"
    exit 1
fi
CDPATH= cd -- "$(dirname -- "$0")"

migration_file="migrations/$(TZ=UTC date +%Y%m%d%H%M%S)_$1.sql"

pgschema plan --file ./schema.sql --output-sql $migration_file
if [[ ! -s $migration_file ]]; then
    rm $migration_file
    echo "There are no diff!"
    exit
fi
echo "Done: $migration_file"
