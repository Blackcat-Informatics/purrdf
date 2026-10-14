# Task 4 outer EXISTS accepted-neighbor diagnostic

The initial persistent projection implementation still copied the complete
projected scope at every nested sub-SELECT when an `EXISTS` token followed the
outer chain. The original unchanged 2,000/4,000/8,000 accepted query test
`nested_star_with_outer_exists_keeps_accepted_scope_work_bounded` failed on
2026-10-13 with these counting-allocator measurements:

| Depth | Allocations | Requested bytes | Retained bytes | Peak working bytes |
| ---: | ---: | ---: | ---: | ---: |
| 2,000 | 20,723,963 | 2,474,573,796 | 194,022,780 | 201,195,878 |
| 4,000 | 90,671,907 | 10,246,449,892 | 772,139,004 | 786,486,118 |
| 8,000 | 394,204,091 | 42,449,917,924 | 3,080,467,452 | 3,109,162,598 |

The unchanged test passed after `last_exists_token` was scoped to each
sub-SELECT's matched opening brace, using the existing token stream:

| Depth | Allocations | Requested bytes | Retained bytes | Peak working bytes |
| ---: | ---: | ---: | ---: | ---: |
| 2,000 | 40,138 | 20,139,252 | 2,640,252 | 9,964,102 |
| 4,000 | 84,103 | 40,470,388 | 5,469,948 | 20,118,470 |
| 8,000 | 176,020 | 81,324,660 | 11,321,340 | 40,619,206 |

Both runs used the standard repository `cargo test` target with one Cargo job,
four test threads and a 4 GiB no-swap systemd scope. These are nonexclusive
allocator quantities, not timing acceptance.
