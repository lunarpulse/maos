# Story 9.4 AC-2 — Homebrew tap formula for MAOS.
#
# Install: brew tap lunarpulse/maos && brew install maos
# The formula re-verifies SHA256 + Ed25519 (fail-closed) by running
# `maosctl install --verify-only --from-local <staged-dir>` post-download.
#
# NOTE: At v0.5 this formula is a SCAFFOLD. The tap repository
# (homebrew-maos) must be created separately. This file serves as the
# template for the first tagged release. The embedded RELEASE_PUBKEY must
# be replaced with the production key before the first tagged release.

class Maos < Formula
  desc "Minimal Agentic Operating System — multi-Spirit runtime"
  homepage "https://github.com/lunarpulse/maos"
  version "0.5.0"
  license "Apache-2.0 OR MIT"

  # Placeholder: replace with the production Ed25519 release public key
  # (64 lowercase hex chars) before the first tagged release.
  RELEASE_PUBKEY = "bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7".freeze

  on_macos do
    on_arm do
      url "https://github.com/lunarpulse/maos/releases/download/v#{version}/maos-darwin-arm64"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/lunarpulse/maos/releases/download/v#{version}/maos-linux-amd64"
      sha256 "PLACEHOLDER_SHA256_LINUX_AMD64"
    end
    on_arm do
      url "https://github.com/lunarpulse/maos/releases/download/v#{version}/maos-linux-arm64"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
  end

  resource "sha256sums" do
    url "https://github.com/lunarpulse/maos/releases/download/v#{version}/SHA256SUMS"
    sha256 "PLACEHOLDER_SHA256_SUMS"
  end

  resource "signature" do
    url "https://github.com/lunarpulse/maos/releases/download/v#{version}/SHA256SUMS.sig"
    sha256 "PLACEHOLDER_SHA256_SIG"
  end

  depends_on "python@3" => :build

  def install
    # Download SHA256SUMS + Ed25519 signature for fail-closed verification.
    resource("sha256sums").stage do
      cp "SHA256SUMS", buildpath/"SHA256SUMS"
    end
    resource("signature").stage do
      cp "SHA256SUMS.sig", buildpath/"SHA256SUMS.sig"
    end

    # Verify the Ed25519 signature over SHA256SUMS using the bundled pubkey.
    # The signature convention is Ed25519(SHA256(SHA256SUMS)) as implemented in
    # `crates/maos-audit/src/release_verify.rs`.
    verify_ed25519_signature(buildpath/"SHA256SUMS", buildpath/"SHA256SUMS.sig", RELEASE_PUBKEY)

    # Verify that the downloaded binary matches the signed manifest.
    binary_name = Dir["maos-*"].first
    expected_hash = sha256sum_for_file(buildpath/"SHA256SUMS", binary_name)
    actual_hash = Digest::SHA256.hexdigest(File.read(binary_name))
    odie "SHA256 mismatch for #{binary_name}" unless expected_hash == actual_hash

    bin.install binary_name => "maos"
  end

  test do
    assert_match "maos", shell_output("#{bin}/maos --version")
  end

  private

  def verify_ed25519_signature(sums_path, sig_path, pubkey_hex)
    verify_script = <<~PYTHON
      import hashlib, sys
      from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
      pubkey = Ed25519PublicKey.from_public_bytes(bytes.fromhex("#{pubkey_hex}"))
      with open("#{sums_path}", "rb") as f:
          message = hashlib.sha256(f.read()).digest()
      with open("#{sig_path}", "rb") as f:
          signature = f.read()
      try:
          pubkey.verify(signature, message)
      except Exception as e:
          print(f"Ed25519 signature verification failed: {e}", file=sys.stderr)
          sys.exit(1)
    PYTHON
    system "python3", "-c", verify_script
  end

  def sha256sum_for_file(sums_path, filename)
    File.readlines(sums_path).each do |line|
      # GNU coreutils format: "<hash>  <filename>"
      hash, name = line.strip.split("  ", 2)
      return hash if name == filename
    end
    odie "#{filename} not found in SHA256SUMS"
  end
end
