# CurtPME Publisher And Timestamp Verification

Date: 2026-07-22

## Objective

Make the protected Windows release package fail closed unless
`translater.exe` has a Windows-valid Authenticode signature, the exact Curt P.
Software leaf identity, and a DigiCert timestamp.

CurtPME remains the signing service and CA namespace. The signer service owns
RFC3161 timestamping through `http://timestamp.digicert.com`; TranslateR keeps
the existing `CURTPME_SIGNER_URL` and `CURTPME_SIGNER_TOKEN` interface and must
not add project-side timestamp configuration.

## Work

- Centralize the Windows Authenticode acceptance policy.
- Enforce it both when signer output is received and independently before the
  release archive is created.
- Add no-signing policy regression tests and run them in Windows CI.
- Reconcile release documentation and checklist evidence.
- Validate and land task-focused local commits only.

## Boundaries

- Do not perform live signing or access signer credentials.
- Do not push, tag, release, publish, or mutate signing infrastructure.
- Remote pipeline proof remains pending until a separately authorized push.
