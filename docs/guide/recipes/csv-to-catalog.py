#!/usr/bin/env python3
"""Export a CSV inventory as the JSON a Writfold catalog reads (#86).

    python3 csv-to-catalog.py equipment.csv > .writfold/catalog-inputs/equipment.json

One object per CSV row under "equipment", keys as the header names them;
a column whose header ends in "[]" is split on ";" into a list. Run it on
the producer's side, review the JSON, and commit it with the catalog's
declaration: Writfold reads the committed JSON, never the CSV, and never
runs this script.
"""
import csv
import json
import sys


def main(path: str) -> int:
    with open(path, newline="", encoding="utf-8") as handle:
        rows = []
        for row in csv.DictReader(handle):
            item = {}
            for key, value in row.items():
                if key.endswith("[]"):
                    item[key[:-2]] = [v.strip() for v in value.split(";") if v.strip()]
                else:
                    item[key] = value.strip()
            rows.append(item)
    json.dump({"equipment": rows}, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1]))
