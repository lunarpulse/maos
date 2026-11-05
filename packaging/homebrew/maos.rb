# Render with xtask render-packages. install uses the independent Python
# verifier before installing any downloaded executable, not maosctl itself.
class Maos < Formula
  desc "Minimal Agentic Operating System"
  homepage "https://github.com/lunarpulse/maos"
  version "@VERSION@"
  license any_of: ["Apache-2.0", "MIT"]
  depends_on "python@3.14" => :build
  depends_on "cryptography" => :build
  # Preserve the signed binaries; :all is deprecated and does not match paths.
  skip_clean "bin/maos", "bin/maosctl", "bin/maos-spirit", "bin/maos-registry-server"

  RELEASES = {
@BREW_RELEASES@
  }.freeze
  SUFFIX = "#{OS.mac? ? "darwin" : "linux"}-#{Hardware::CPU.arm? ? "arm64" : "amd64"}".freeze
  odie "This release has no binaries for #{SUFFIX}" unless RELEASES.key?(SUFFIX)
  url "@BASE_URL@/maos-#{SUFFIX}", using: :nounzip
  sha256 RELEASES.fetch(SUFFIX).fetch(0)

  %w[maosctl maos-spirit maos-registry-server].each_with_index do |binary, index|
    resource binary do
      url "@BASE_URL@/#{binary}-#{SUFFIX}", using: :nounzip
      sha256 RELEASES.fetch(SUFFIX).fetch(index + 1)
    end
  end
  resource "manifest" do
    url "@BASE_URL@/SHA256SUMS", using: :nounzip
    sha256 "@SHA256SUMS_HASH@"
  end
  resource "signature" do
    url "@BASE_URL@/SHA256SUMS.sig", using: :nounzip
    sha256 "@SIGNATURE_HASH@"
  end
  resource "verifier" do
    url "@BASE_URL@/verify-release.py", using: :nounzip
    sha256 "@VERIFIER_HASH@"
  end

  def install
    resources.each do |resource|
      resource.stage { buildpath.install Dir["*"] }
    end
    # Homebrew's cryptography and cffi formulae link into this site-packages.
    ENV.prepend_path "PYTHONPATH", HOMEBREW_PREFIX/"lib/python3.14/site-packages"
    names = %w[maos maosctl maos-spirit maos-registry-server]
    system Formula["python@3.14"].opt_bin/"python3.14", "verify-release.py",
           "--pubkey", "@PUBKEY@", *names.map { |name| "#{name}-#{SUFFIX}" }
    names.each { |name| bin.install "#{name}-#{SUFFIX}" => name }
  end

  test do
    assert_match "maos", shell_output("#{bin}/maos --version")
  end
end
