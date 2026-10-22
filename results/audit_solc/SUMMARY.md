# Target A Audit — Solidity Compiler (solc) 0.8.24 `--import-ast`

## Environment
- `solc, the solidity compiler commandline interface / Version: 0.8.24+commit.e11b9ed9.Linux.g++` installed at `.tools/bin/solc`
- Fixture: `test.sol` (`Vault` contract with `uint256 constant MAX = 100`, `deposit`).

## 1. Export
```
$ solc --combined-json ast --pretty-json test.sol > ast.json
```
Type-checked metadata is serialized **directly into the JSON artifact**:
- 84 AST nodes, 34 nodes carrying `typeDescriptions`
- `typeIdentifier` occurrences: 38, `isConstant`: 11, `lValueRequested`: 11
- Sample (`VariableDeclaration MAX`):
  ```json
  "typeDescriptions": { "typeIdentifier": "t_uint256", "typeString": "uint256" },
  "value": { ..., "isConstant": false, "isPure": true, "lValueRequested": false,
             "typeDescriptions": { "typeIdentifier": "t_rational_100_by_1",
                                   "typeString": "int_const 100" } }
  ```

## 2. Mutation experiment

Import format accepted by `--import-ast` (per `solc --help`): "the output of the
--standard-json or the one produced by --combined-json ast". The passing format
was the standard-json wrapper:
`{"language":"Solidity","sources":{"test.sol":{"AST": <artifact AST>}}}`

| Mutation (structurally valid unless noted) | Result |
|---|---|
| M1: dropped `id`/`nodeType` from one node | Rejected: `Failed to import AST: JSON-Node needs to have 'nodeType' and 'id' fields.` (structural check) |
| M2 (`import_mutC_literal.json`): Literal for MAX changed `value` 100→999999 with **stale** `typeString: "int_const 100"` | Rejected: `Failed to import AST: Analysis of the AST failed` — solc **re-runs analysis** and catches the semantic inconsistency |
| M3 (`import_mutB_typelie.json`): `VariableDeclaration newBal` `typeDescriptions.typeIdentifier` uint256→bool (self-consistent with a corrupted `typeName`) | **Accepted**, compiled to Yul. Bytecode identical to the honest compile except the solc metadata fingerprint (CBOR bzzr hash) — i.e. **no codegen effect**. |

## 3. Finding

`solc --import-ast` does NOT trust the serialized `typeDescriptions*`:
the AST importer reconstructs the source AST and re-runs the type checker; the
embedded semantic metadata is ignored (M3 had zero codegen effect) and
inconsistent metadata aborts compilation (M2). The one codegen-visible change
between honest and mutated runs is the *compiler metadata fingerprint keccak*, not
generated code.

This is the defense present in solc's TOFU path that Wasmtime's `.cwasm` PATH
lacks (see `../audit_wasmtime/AUDIT.md`): solc re-verifies semantics at the
consumer; Wasmtime re-hydrates compiled code with only a metadata version check
and no content authentication.
