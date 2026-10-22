#!/usr/bin/env python3
"""Walk an solc --combined-json ast output and summarize type-checked metadata.

Used for the paper's Section 6.1.1: demonstrates that solc serializes
full type-checked semantic metadata (typeDescriptions, isConstant,
lValueRequested, typeIdentifier) directly into the JSON artifact.
"""
import json
import sys


def iter_nodes(n):
    """Generic recursive walk over any JSON structure, yielding dicts."""
    if isinstance(n, dict):
        yield n
        for v in n.values():
            yield from iter_nodes(v)
    elif isinstance(n, list):
        for v in n:
            yield from iter_nodes(v)


def main():
    d = json.load(open(sys.argv[1]))
    sources = d["sources"]
    print("solc version field:", d.get("version"))
    for path, src in sources.items():
        ast = src["AST"]
        nodes = list(iter_nodes(ast))
        typed = [n for n in nodes if "typeDescriptions" in n]
        print(f"\nSource: {path}")
        print(f"  total AST nodes:                 {len(nodes)}")
        print(f"  nodes with typeDescriptions:     {len(typed)}")
        print(f"  occ isConstant:                  {json.dumps(ast).count('\"isConstant\"')}")
        print(f"  occ lValueRequested:             {len([x for x in nodes if 'lValueRequested' in x])}")
        print(f"  occ typeIdentifier:              {json.dumps(ast).count('typeIdentifier')}")

        decls = [n for n in nodes if n.get("nodeType") == "VariableDeclaration"]
        if decls:
            sample = next(n for n in decls if "typeDescriptions" in n)
            print("\n  Sample VariableDeclaration (type-checked metadata embedded):")
            print("   ", json.dumps(sample, indent=4))


if __name__ == "__main__":
    main()
