export interface Platform {
  id: string;
  os: string;
  osIcon: string;
  arch: string;
  archFull: string;
  variant?: string;
  displayName: string;
}

export interface Asset {
  filename: string;
  platform: Platform;
  type: "binary" | "deb" | "rpm";
  extension: string;
  size?: number;
  sha256?: string;
}

export interface AssetGroup {
  title: string;
  description: string;
  assets: Asset[];
}

export interface BuildStatus {
  build?: "success" | "failure" | "cancelled" | "skipped";
  "package-deb"?: "success" | "failure" | "cancelled" | "skipped";
  "package-rpm"?: "success" | "failure" | "cancelled" | "skipped";
}

export interface ReleaseContext {
  version: string;
  versionClean: string;
  projectName: string;
  projectDescription: string;
  repository: string;
  repositoryUrl: string;
  isPrerelease: boolean;
  isNightly: boolean;
  date: string;
  dateIso: string;

  // Asset groups
  binaries: Asset[];
  debPackages: Asset[];
  rpmPackages: Asset[];

  // Organized by OS
  linuxAssets: Asset[];
  macosAssets: Asset[];
  windowsAssets: Asset[];

  // Status
  hasFailures: boolean;
  buildStatus: BuildStatus;
  workflowRunUrl?: string;

  // Install commands
  installCommands: InstallCommand[];
}

export interface InstallCommand {
  os: string;
  icon: string;
  methods: InstallMethod[];
}

export interface InstallMethod {
  name: string;
  command: string;
  note?: string;
}

// Platform detection patterns
export const PLATFORM_PATTERNS: Record<string, Partial<Platform>> = {
  "linux-amd64-musl": {
    os: "Linux",
    osIcon: "🐧",
    arch: "x86_64",
    archFull: "x86_64 (64-bit, static)",
    variant: "musl",
    displayName: "Linux x86_64 (static)",
  },
  "linux-amd64": {
    os: "Linux",
    osIcon: "🐧",
    arch: "x86_64",
    archFull: "x86_64 (64-bit)",
    displayName: "Linux x86_64",
  },
  "linux-aarch64": {
    os: "Linux",
    osIcon: "🐧",
    arch: "aarch64",
    archFull: "ARM64",
    displayName: "Linux ARM64",
  },
  "darwin-arm64": {
    os: "macOS",
    osIcon: "🍎",
    arch: "arm64",
    archFull: "Apple Silicon (M1/M2/M3/M4)",
    displayName: "macOS Apple Silicon",
  },
  "darwin-amd64": {
    os: "macOS",
    osIcon: "🍎",
    arch: "x86_64",
    archFull: "Intel (64-bit)",
    displayName: "macOS Intel",
  },
  "windows-amd64": {
    os: "Windows",
    osIcon: "🪟",
    arch: "x86_64",
    archFull: "x86_64 (64-bit)",
    displayName: "Windows x86_64",
  },
  "windows-aarch64": {
    os: "Windows",
    osIcon: "🪟",
    arch: "arm64",
    archFull: "ARM64",
    displayName: "Windows ARM64",
  },
};

// DEB architecture mapping
export const DEB_ARCH_MAP: Record<string, string> = {
  amd64: "linux-amd64",
  arm64: "linux-aarch64",
};

// RPM architecture mapping
export const RPM_ARCH_MAP: Record<string, string> = {
  x86_64: "linux-amd64",
  aarch64: "linux-aarch64",
};
