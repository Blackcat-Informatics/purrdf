# Lossless stack evidence packaging

Each original text export below was scanned with the exact current ghprsq credential pattern before compression. The source text was compressed with `zstd -T1 -3`; decompression was independently streamed through SHA-256 and matched the original digest before the original file was removed. This reduces archive size without removing evidence. Existing heaptrack binary captures remain separate inputs; they are not assumed to reproduce these rendered exports.

Replay any original export with `zstd -d -c FILE.zst > ORIGINAL_NAME`. Historical references to the original text paths resolve through this map.

| Original | Archived | Original bytes | Archived bytes | Original and decoded SHA-256 |
| --- | --- | ---: | ---: | --- |
| `sparql-alloc-baseline-stacks.txt` | `sparql-alloc-baseline-stacks.txt.zst` | 51230433 | 157654 | `a568aab5e4ae38d1ddd2baae8995ca96562e2279d40023556a630a96e99a6eec` |
| `sparql-alloc-current-stacks.txt` | `sparql-alloc-current-stacks.txt.zst` | 56880186 | 190511 | `5b88b1390cf3b35228fd21707a4c044418437ba244f394762fd0ec30bbb7ea9b` |
| `sparql-final-baseline.demangled.stacks` | `sparql-final-baseline.demangled.stacks.zst` | 99763639 | 196165 | `e49005ef665a7ee9f19472cabe9b0df7a2e3e7568937b7488bebc55f53d0cc39` |
| `sparql-final-baseline.stacks` | `sparql-final-baseline.stacks.zst` | 50701522 | 155725 | `ee9f3d70312271a832ba952f0ab064d8a8220bda26499a800b10588f343ab8d7` |
| `sparql-final-current.demangled.stacks` | `sparql-final-current.demangled.stacks.zst` | 131021523 | 291020 | `31b2beb2318bc49c01e1fa75db4dcea371a94d1054ee5f7352fd52b661f15525` |
| `sparql-final-current.stacks` | `sparql-final-current.stacks.zst` | 65396145 | 209531 | `190fba1d40b81355957e72926df681e924c9096f1a319d3d917c8878cb642056` |
