# Fixed-work instruction acceptance

These runs measure retired user instructions, never elapsed time. The matrix
uses 1,024, 16,384 and 262,144 reifiers, six annotations each, with constant
matching populations: one target, two component matches, three annotation
matches. Each population/kind has five interleaved control/query pairs. Both
modes construct and warm the same data and prepared queries. Subtract control
instructions before dividing by fixed iterations (1,048,576 target lookups or
8,192 complete queries). Controls accumulate the known count; queries perform
the real production operation. Require positive paired deltas and full event
coverage. Report all five values, their median, and the largest/smallest
population median ratio. The previously selected conservative sublinear bound
is at most 3x instructions across 256x population growth.

The release executable is recorded in component-qualified-executable.txt and
its producing source in component-scale-source.patch. The later fixture move
shares the identical generator/query bodies with the portable target; it changes
no production implementation, input population or query. Raw CSV and libtest
logs are retained under instruction-matrix. Time fields are not acceptance data.
