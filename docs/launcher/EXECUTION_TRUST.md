# Windows execution trust for Ooze

The Windows x64 RC1 builds are unsigned until a signer is provisioned. Zip
hashes verify the package bytes. Windows execution trust and Minecraft Services
application access each have their own requirements.

| Option | Cost | Setup and limits |
|---|---|---|
| Authenticode with a trusted CA certificate | Microsoft estimates US$150–300 per year for OV signing. Check the CA quote, eligibility and any hardware or service fees. | Obtain a code-signing certificate from a CA in Microsoft's trusted root program. Complete the CA's identity validation, provision the protected signing key in its hardware or cloud service, and configure SignTool to sign and RFC 3161 timestamp the GUI, CLI and native DLL. Verify each signature with `signtool verify /pa /all /v`. [Microsoft code-signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) |
| Azure Artifact Signing, formerly Trusted Signing | Microsoft estimates US$9.99 per month for Basic, which includes 5,000 signatures per month. Confirm regional prices and overage before provisioning. | Create the Azure Artifact Signing account, pass identity validation and create a **Public Trust** certificate profile. Eligibility depends on identity type and region. For GitHub Actions, use a dedicated workload identity federated to the release environment and grant **Artifact Signing Certificate Profile Signer** at the profile scope. Use Microsoft's Windows signing action with RFC 3161 timestamping. Keep identity configuration in protected CI variables or secrets, and use OIDC for access. [Pricing](https://azure.microsoft.com/en-us/pricing/details/artifact-signing/) · [Account setup](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart) · [GitHub OIDC setup](https://github.com/Azure/artifact-signing-action/blob/main/docs/OIDC.md) |
| Local administrator-approved App Control for Business rule | A file rule itself has no signing subscription cost. The administrator is responsible for management and deployment. | On a device where an administrator owns an App Control for Business policy, verify the exact RC1 ZIP and binaries. The administrator can use the App Control Wizard to create a supplemental policy for an eligible managed base policy, generate PE image hash rules from the verified EXEs and DLL, audit the results and deploy through the approved management channel. A rebuild requires new hash rules. Stock Smart App Control has no per-app allowlist. [Supplemental App Control policy](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/design/appcontrol-wizard-create-supplemental-policy) · [Smart App Control FAQ](https://support.microsoft.com/en-gb/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions) |

Sign the DLL before generating the native metadata and building the mod JAR.
Sign both EXEs before computing the package SHA256SUMS and RC1 ZIP digest.
Publish the resulting hashes and rerun the affected acceptance steps against
the signed candidate. A valid signature identifies the publisher. SmartScreen
can still show a reputation warning for a new publisher.

Keep signing keys and passwords out of chat, Git history, workflow output and
packaged files. Use the configured CI secret store or hardware signing service
for private material.
