# SunoDM local verification patch

This directory vendors `sigstore-tsa` 0.10.0 from
`prefix-dev/sigstore-rust`, upstream commit
`2501a347c5c858bb91feb96f40f8eb67f06d6418` (`crates/sigstore-tsa`).

SunoDM keeps the upstream parser, CMS signed-attribute checks, and WebPKI
chain validation together, and applies a narrow local verification patch:

- dispatch CMS signatures by the declared signature algorithm;
- accept ECDSA plus RSA PKCS#1 v1.5 and parameter-validated RSA-PSS with
  SHA-256/384/512;
- reject unsupported or inconsistent signature-algorithm declarations;
- require the TSA signer's Extended Key Usage to be critical and contain
  exactly `id-kp-timeStamping`, as required by RFC 3161.

The upstream code and this patch are distributed under Apache-2.0; see
`LICENSE`. Keep the crate version at `0.10.0` so Cargo metadata and archived
verification reports identify the upstream base accurately. SunoDM's
application-level verifier label additionally marks the local strict patch.
