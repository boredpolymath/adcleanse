# Security Policy & Vulnerability Reporting
### Bored Polymath Studios — AdCleanse

At Bored Polymath Studios, the security and privacy of our users are foundational. Because AdCleanse operates as an offline-first privacy console interacting with user credentials and local encrypted storage, we take reports of security flaws seriously.

---

## Supported Versions

Only the latest active release on the main branch receives active security updates and vulnerability patches.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

---

## Reporting a Vulnerability

We strongly encourage responsible, coordinated vulnerability disclosure.

### 1. Private Vulnerability Reporting (Preferred)
Please submit security vulnerabilities privately using GitHub's Private Vulnerability Reporting feature:
* **Direct Link:** [Open Private Advisory](https://github.com/boredpolymath/adcleanse/security/advisories/new)

This allows us to review, collaborate on a patch, and coordinate a secure fix before public disclosure.

### 2. Direct Contact
If you cannot use GitHub's advisory system, you may contact the maintainers directly:
* **Email:** boredpolymath@proton.me
* **Subject:** `[SECURITY] AdCleanse Vulnerability Report`

---

## What to Include in Your Report
To help us triage and resolve your report quickly, please include:
1. **Description**: Clear summary of the vulnerability and its potential impact.
2. **Steps to Reproduce**: Detailed reproduction steps, sample payload, or minimal proof of concept (PoC).
3. **Environment**: Target operating system (macOS / Windows / Linux), Tauri version, and build architecture.
4. **Proposed Fix**: Any suggested mitigations or patches (optional).

---

## Response Timeline
* **Initial Acknowledgment**: Within 48 hours of receipt.
* **Assessment & Triage**: Within 5 business days.
* **Remediation & Release**: Coordinated fix deployed via a signed release tag and release advisory.
