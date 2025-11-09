# Security Policy

We take the security of this project seriously and appreciate responsible disclosures from the community.

If you believe you have found a security vulnerability, **please do not create a public issue**. Instead, follow the process below.

---

## Supported Versions

We aim to provide security fixes for actively used branches and the latest tagged releases.

| Version / Branch | Supported           |
| ---------------- | ------------------- |
| `main`           | ✅ Active           |
| Latest `0.x` tag | ✅ Security fixes\* |
| Older releases   | ❌ Not supported    |

\*Security fixes may be backported on a best-effort basis.

These ranges are indicative — check the Git history and releases for the most up-to-date view.

---

## Reporting a Vulnerability

If you discover a vulnerability or potential security issue:

1. **Email us privately** with details of the issue
   - Subject line: `Security report: <short title>`
   - Include:
     - A clear description of the issue and where it occurs
     - Steps to reproduce (if possible)
     - Any relevant logs, configuration snippets, or PoCs
   - Please avoid including secrets, private keys, or any personal data in the report.

2. Allow us **a reasonable amount of time** to investigate and patch the issue before you disclose it publicly.

> Replace this with your actual contact:
>
> - `security@example.com`
> - or GitHub security advisories if you prefer that flow.

---

## What to Expect

After you report a vulnerability:

- **Acknowledgement**  
  We will acknowledge receipt of your report as soon as possible (typically within a few business days).

- **Triage & Investigation**  
  We will:
  - Verify the issue
  - Assess severity and impact
  - Identify affected configurations / versions

- **Fix & Release Plan**  
  For valid issues, we will:
  - Develop a fix or mitigation
  - Prepare tests to prevent regressions
  - Plan an appropriate release or patch, and, where feasible, backport to supported versions

- **Disclosure**  
  Once a fix is available, we will:
  - Publish release notes or a changelog entry describing the issue at a high level
  - Give credit to you (if you wish) for responsible disclosure

---

## Out of Scope

The following are generally considered out of scope for this project’s security policy:

- Issues that require:
  - Non-default, unsafe configuration
  - Local access to the host with elevated privileges
- Denial of service caused by:
  - Extremely large or maliciously crafted inputs beyond realistic deployment expectations
- Vulnerabilities in **third-party dependencies** that are not exploitable through this project’s binaries/services (though we still appreciate heads-up reports)

---

## Best Practices for Running This Project

While not strictly part of the vulnerability process, we recommend:

- Running services behind a **reverse proxy** (e.g. Nginx/Envoy/Traefik) with TLS termination
- Using **separate credentials** for dev, staging, and production environments
- Restricting access to:
  - Chain node RPC ports
  - Internal ML service endpoints
- Rotating keys and secrets regularly (including validator keys and ML artefact credentials)
- Keeping Docker images and dependencies up to date

Thank you for helping keep this project and its users safe.
