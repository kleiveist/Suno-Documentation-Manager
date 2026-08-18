# Historical document fixtures (not current goldens)

The files below this directory are retained only as historical template-1.7 / workflow-1.3
examples. No current test consumes them, and they must not be treated as expected output for the
current template, workflow, manifest, or certificate formats.

Current output behaviour is specified and verified by the renderer and end-to-end tests in
`src-tauri/src/documents.rs`, `src-tauri/src/certificate.rs`, `src-tauri/src/certificate_pdf.rs`,
and `src-tauri/src/application.rs`.

Do not update these historical bytes to make a current test pass. If they are used again, move them
to an explicitly versioned migration fixture and add a test that states the historical version it
is exercising.
