# Bridge and FFI

This chapter is for contributors working on `cbf-chrome-sys` and Chromium-side bridge code.

Key invariants to preserve:

- no raw `WebContents*` across async boundaries
- use stable IDs and re-resolve at execution time
- guard callbacks with weak ownership
- tolerate duplicate, late, or failed close paths
- keep Chromium-specific details out of the public `cbf` API

This is where the detailed implementation constraints should live in a structured, contributor-focused form.
