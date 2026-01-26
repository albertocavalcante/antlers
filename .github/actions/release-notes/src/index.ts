import * as core from "@actions/core";
import * as glob from "@actions/glob";
import * as fs from "fs";
import * as path from "path";
import Handlebars from "handlebars";
import {
  Asset,
  BuildStatus,
  InstallCommand,
  Platform,
  PLATFORM_PATTERNS,
  DEB_ARCH_MAP,
  RPM_ARCH_MAP,
  ReleaseContext,
} from "./types";

// Register Handlebars helpers
Handlebars.registerHelper("eq", (a, b) => a === b);
Handlebars.registerHelper("neq", (a, b) => a !== b);
Handlebars.registerHelper("or", (...args) => args.slice(0, -1).some(Boolean));
Handlebars.registerHelper("and", (...args) => args.slice(0, -1).every(Boolean));
Handlebars.registerHelper("not", (a) => !a);
Handlebars.registerHelper("gt", (a, b) => a > b);
Handlebars.registerHelper("len", (arr) =>
  Array.isArray(arr) ? arr.length : 0,
);
Handlebars.registerHelper(
  "hasItems",
  (arr) => Array.isArray(arr) && arr.length > 0,
);
Handlebars.registerHelper("lowercase", (str) => String(str).toLowerCase());
Handlebars.registerHelper("uppercase", (str) => String(str).toUpperCase());
Handlebars.registerHelper("truncate", (str, len) => String(str).slice(0, len));

/**
 * Detect platform from filename
 */
function detectPlatform(
  filename: string,
  projectName: string,
): Platform | null {
  const basename = path.basename(filename);

  // Try to match known patterns
  // Binary: antlers-linux-amd64.tar.gz, antlers-windows-amd64.zip
  // DEB: antlers_0.1.0_amd64.deb
  // RPM: antlers-0.1.0-1.x86_64.rpm

  // Check for binary patterns first (most specific first)
  for (const [id, platform] of Object.entries(PLATFORM_PATTERNS)) {
    if (basename.includes(id)) {
      return { id, ...platform } as Platform;
    }
  }

  // Check for DEB pattern: _arch.deb
  const debMatch = basename.match(/_([a-z0-9]+)\.deb$/);
  if (debMatch) {
    const debArch = debMatch[1];
    const platformId = DEB_ARCH_MAP[debArch];
    if (platformId && PLATFORM_PATTERNS[platformId]) {
      return { id: platformId, ...PLATFORM_PATTERNS[platformId] } as Platform;
    }
  }

  // Check for RPM pattern: .arch.rpm
  const rpmMatch = basename.match(/\.([a-z0-9_]+)\.rpm$/);
  if (rpmMatch) {
    const rpmArch = rpmMatch[1];
    const platformId = RPM_ARCH_MAP[rpmArch];
    if (platformId && PLATFORM_PATTERNS[platformId]) {
      return { id: platformId, ...PLATFORM_PATTERNS[platformId] } as Platform;
    }
  }

  return null;
}

/**
 * Determine asset type from extension
 */
function getAssetType(filename: string): Asset["type"] {
  if (filename.endsWith(".deb")) return "deb";
  if (filename.endsWith(".rpm")) return "rpm";
  return "binary";
}

/**
 * Get file extension
 */
function getExtension(filename: string): string {
  if (filename.endsWith(".tar.gz")) return "tar.gz";
  if (filename.endsWith(".tar.xz")) return "tar.xz";
  const ext = path.extname(filename);
  return ext.startsWith(".") ? ext.slice(1) : ext;
}

/**
 * Read SHA256 checksum from .sha256 file
 */
async function readSha256(assetPath: string): Promise<string | undefined> {
  const sha256Path = `${assetPath}.sha256`;
  try {
    const content = await fs.promises.readFile(sha256Path, "utf-8");
    // Format: "hash  filename" or just "hash"
    const match = content.trim().match(/^([a-f0-9]{64})/i);
    return match?.[1]?.toLowerCase();
  } catch {
    return undefined;
  }
}

/**
 * Get file size
 */
async function getFileSize(filepath: string): Promise<number | undefined> {
  try {
    const stats = await fs.promises.stat(filepath);
    return stats.size;
  } catch {
    return undefined;
  }
}

/**
 * Format bytes to human readable
 */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// Register the formatBytes helper
Handlebars.registerHelper("formatBytes", formatBytes);

/**
 * Discover all release assets
 */
async function discoverAssets(
  assetsDir: string,
  projectName: string,
): Promise<Asset[]> {
  const assets: Asset[] = [];

  const patterns = [
    `${assetsDir}/**/*.tar.gz`,
    `${assetsDir}/**/*.zip`,
    `${assetsDir}/**/*.deb`,
    `${assetsDir}/**/*.rpm`,
  ];

  const globber = await glob.create(patterns.join("\n"), {
    followSymbolicLinks: false,
  });
  const files = await globber.glob();

  for (const filepath of files) {
    // Skip checksum files
    if (filepath.endsWith(".sha256")) continue;

    const filename = path.basename(filepath);
    const platform = detectPlatform(filename, projectName);

    if (!platform) {
      core.warning(`Could not detect platform for: ${filename}`);
      continue;
    }

    const asset: Asset = {
      filename,
      platform,
      type: getAssetType(filename),
      extension: getExtension(filename),
      size: await getFileSize(filepath),
      sha256: await readSha256(filepath),
    };

    assets.push(asset);
  }

  // Sort assets by platform for consistent ordering
  const platformOrder = [
    "linux-amd64",
    "linux-amd64-musl",
    "linux-aarch64",
    "darwin-arm64",
    "darwin-amd64",
    "windows-amd64",
  ];
  assets.sort((a, b) => {
    const aIndex = platformOrder.indexOf(a.platform.id);
    const bIndex = platformOrder.indexOf(b.platform.id);
    return (aIndex === -1 ? 999 : aIndex) - (bIndex === -1 ? 999 : bIndex);
  });

  return assets;
}

/**
 * Generate install commands for a version
 */
function generateInstallCommands(
  version: string,
  projectName: string,
  repository: string,
): InstallCommand[] {
  const cleanVersion = version.replace(/^v/, "");
  const isNightly = version === "nightly" || version.includes("nightly");

  const commands: InstallCommand[] = [
    {
      os: "Linux",
      icon: "",
      methods: [
        {
          name: "Shell (x86_64)",
          command: `curl -fsSL https://github.com/${repository}/releases/download/${version}/${projectName}-linux-amd64.tar.gz | tar xz\nsudo mv ${projectName} /usr/local/bin/`,
        },
        {
          name: "Shell (ARM64)",
          command: `curl -fsSL https://github.com/${repository}/releases/download/${version}/${projectName}-linux-aarch64.tar.gz | tar xz\nsudo mv ${projectName} /usr/local/bin/`,
        },
        {
          name: "Debian/Ubuntu",
          command: `curl -fsSLO https://github.com/${repository}/releases/download/${version}/${projectName}_${cleanVersion}_amd64.deb\nsudo dpkg -i ${projectName}_${cleanVersion}_amd64.deb`,
          note: isNightly
            ? "Version format may differ for nightly builds"
            : undefined,
        },
        {
          name: "RHEL/Fedora",
          command: `sudo rpm -i https://github.com/${repository}/releases/download/${version}/${projectName}-${cleanVersion}-1.x86_64.rpm`,
          note: isNightly
            ? "Version format may differ for nightly builds"
            : undefined,
        },
        {
          name: "Cargo",
          command: `cargo install ${projectName}-cli`,
          note: "Installs latest stable from crates.io",
        },
      ],
    },
    {
      os: "macOS",
      icon: "",
      methods: [
        {
          name: "Shell (Apple Silicon)",
          command: `curl -fsSL https://github.com/${repository}/releases/download/${version}/${projectName}-darwin-arm64.tar.gz | tar xz\nsudo mv ${projectName} /usr/local/bin/`,
        },
        {
          name: "Shell (Intel)",
          command: `curl -fsSL https://github.com/${repository}/releases/download/${version}/${projectName}-darwin-amd64.tar.gz | tar xz\nsudo mv ${projectName} /usr/local/bin/`,
        },
        {
          name: "Cargo",
          command: `cargo install ${projectName}-cli`,
          note: "Installs latest stable from crates.io",
        },
      ],
    },
    {
      os: "Windows",
      icon: "",
      methods: [
        {
          name: "PowerShell",
          command: `Invoke-WebRequest -Uri "https://github.com/${repository}/releases/download/${version}/${projectName}-windows-amd64.zip" -OutFile "${projectName}.zip"\nExpand-Archive -Path "${projectName}.zip" -DestinationPath .\nMove-Item -Path ".\\${projectName}.exe" -Destination "$env:LOCALAPPDATA\\Microsoft\\WindowsApps\\"`,
        },
        {
          name: "Cargo",
          command: `cargo install ${projectName}-cli`,
          note: "Installs latest stable from crates.io",
        },
      ],
    },
  ];

  return commands;
}

/**
 * Build the release context
 */
async function buildContext(
  version: string,
  assetsDir: string,
  projectName: string,
  projectDescription: string,
  repository: string,
  isPrerelease: boolean,
  buildStatus: BuildStatus,
  workflowRunId: string,
): Promise<ReleaseContext> {
  const assets = await discoverAssets(assetsDir, projectName);

  const binaries = assets.filter((a) => a.type === "binary");
  const debPackages = assets.filter((a) => a.type === "deb");
  const rpmPackages = assets.filter((a) => a.type === "rpm");

  const hasFailures = Object.values(buildStatus).some((s) => s === "failure");

  const now = new Date();

  return {
    version,
    versionClean: version.replace(/^v/, ""),
    projectName,
    projectDescription,
    repository,
    repositoryUrl: `https://github.com/${repository}`,
    isPrerelease,
    isNightly: version === "nightly" || version.includes("nightly"),
    date: now.toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric",
    }),
    dateIso: now.toISOString().split("T")[0],

    binaries,
    debPackages,
    rpmPackages,

    linuxAssets: assets.filter((a) => a.platform.os === "Linux"),
    macosAssets: assets.filter((a) => a.platform.os === "macOS"),
    windowsAssets: assets.filter((a) => a.platform.os === "Windows"),

    hasFailures,
    buildStatus,
    workflowRunUrl: workflowRunId
      ? `https://github.com/${repository}/actions/runs/${workflowRunId}`
      : undefined,

    installCommands: generateInstallCommands(version, projectName, repository),
  };
}

/**
 * Default template
 */
const DEFAULT_TEMPLATE = `{{#if isNightly}}
> **Nightly Build** — This is an automated build from the latest code and may be unstable.
> For production use, please use a [stable release]({{repositoryUrl}}/releases/latest).

{{/if}}
{{#if isPrerelease}}
{{#unless isNightly}}
> **Pre-release** — This version is not yet considered stable.

{{/unless}}
{{/if}}
{{projectDescription}}

## Installation

{{#each installCommands}}
<details>
<summary><strong>{{os}}</strong></summary>

{{#each methods}}
**{{name}}**
\`\`\`bash
{{{command}}}
\`\`\`
{{#if note}}
<sub>{{note}}</sub>
{{/if}}

{{/each}}
</details>

{{/each}}

## Downloads

{{#if (hasItems binaries)}}
### Binaries

| Platform | Architecture | Download | Size | SHA256 |
|----------|--------------|----------|------|--------|
{{#each binaries}}
| {{platform.osIcon}} {{platform.os}} | {{platform.archFull}} | [\`{{filename}}\`]({{../repositoryUrl}}/releases/download/{{../version}}/{{filename}}) | {{#if size}}{{formatBytes size}}{{else}}—{{/if}} | {{#if sha256}}\`{{truncate sha256 12}}…\`{{else}}—{{/if}} |
{{/each}}

{{/if}}
{{#if (hasItems debPackages)}}
### DEB Packages (Debian, Ubuntu)

| Architecture | Download | Size | SHA256 |
|--------------|----------|------|--------|
{{#each debPackages}}
| {{platform.archFull}} | [\`{{filename}}\`]({{../repositoryUrl}}/releases/download/{{../version}}/{{filename}}) | {{#if size}}{{formatBytes size}}{{else}}—{{/if}} | {{#if sha256}}\`{{truncate sha256 12}}…\`{{else}}—{{/if}} |
{{/each}}

{{/if}}
{{#if (hasItems rpmPackages)}}
### RPM Packages (RHEL, Fedora, CentOS)

| Architecture | Download | Size | SHA256 |
|--------------|----------|------|--------|
{{#each rpmPackages}}
| {{platform.archFull}} | [\`{{filename}}\`]({{../repositoryUrl}}/releases/download/{{../version}}/{{filename}}) | {{#if size}}{{formatBytes size}}{{else}}—{{/if}} | {{#if sha256}}\`{{truncate sha256 12}}…\`{{else}}—{{/if}} |
{{/each}}

{{/if}}
## Checksums

All downloads include SHA256 checksum files (\`.sha256\`) for verification:

\`\`\`bash
# Verify after download
shasum -a 256 -c {{projectName}}-<platform>.tar.gz.sha256
\`\`\`

{{#if hasFailures}}
---

> **Note:** Some builds failed. See [workflow run]({{workflowRunUrl}}) for details.
{{/if}}
`;

/**
 * Main action
 */
async function run(): Promise<void> {
  try {
    // Get inputs
    const version = core.getInput("version", { required: true });
    const assetsDir = core.getInput("assets-dir", { required: true });
    const projectName = core.getInput("project-name") || "antlers";
    const projectDescription = core.getInput("project-description") || "";
    const repository =
      core.getInput("repository") || process.env.GITHUB_REPOSITORY || "";
    const isPrerelease = core.getInput("is-prerelease") === "true";
    const workflowRunId =
      core.getInput("workflow-run-id") || process.env.GITHUB_RUN_ID || "";
    const customTemplatePath = core.getInput("template");

    // Parse build status
    let buildStatus: BuildStatus = {};
    try {
      const statusInput = core.getInput("build-status");
      if (statusInput && statusInput !== "{}") {
        buildStatus = JSON.parse(statusInput);
      }
    } catch (e) {
      core.warning(`Failed to parse build-status: ${e}`);
    }

    core.info(`Generating release notes for ${version}`);
    core.info(`Assets directory: ${assetsDir}`);

    // Build context
    const context = await buildContext(
      version,
      assetsDir,
      projectName,
      projectDescription,
      repository,
      isPrerelease,
      buildStatus,
      workflowRunId,
    );

    core.info(
      `Found ${context.binaries.length} binaries, ${context.debPackages.length} DEBs, ${context.rpmPackages.length} RPMs`,
    );

    // Load template
    let templateSource = DEFAULT_TEMPLATE;
    if (customTemplatePath) {
      try {
        templateSource = await fs.promises.readFile(
          customTemplatePath,
          "utf-8",
        );
        core.info(`Using custom template: ${customTemplatePath}`);
      } catch (e) {
        core.warning(`Failed to load custom template, using default: ${e}`);
      }
    }

    // Compile and render
    const template = Handlebars.compile(templateSource);
    const releaseNotes = template(context);

    // Write to file
    const outputPath = path.join(process.cwd(), "release-notes.md");
    await fs.promises.writeFile(outputPath, releaseNotes);

    // Set outputs
    core.setOutput("release-notes", releaseNotes);
    core.setOutput("release-notes-file", outputPath);

    core.info(`Release notes generated successfully: ${outputPath}`);
  } catch (error) {
    if (error instanceof Error) {
      core.setFailed(error.message);
    } else {
      core.setFailed("An unexpected error occurred");
    }
  }
}

run();
