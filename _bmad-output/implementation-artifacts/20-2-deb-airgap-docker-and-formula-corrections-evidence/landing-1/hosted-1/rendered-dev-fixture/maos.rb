# Render with xtask render-packages. install uses the independent Python
# verifier before installing any downloaded executable, not maosctl itself.
class Maos < Formula
  desc "Minimal Agentic Operating System"
  homepage "https://github.com/lunarpulse/maos"
  version "0.1.0-alpha.1"
  license any_of: ["Apache-2.0", "MIT"]
  depends_on "python@3.14" => :build
  depends_on "cryptography" => :build
  # Preserve the signed binaries; :all is deprecated and does not match paths.
  skip_clean "bin/maos", "bin/maosctl", "bin/maos-spirit", "bin/maos-registry-server"

  RELEASES = {
    "linux-amd64" => ["ff4a9adc11c441f4af6d1bd04c6a0e6d9ae1cf54fb9ed5b693b2c1848854b6ad", "acf6bc69e5c719e0066fee78b9f48f037c7712cb9c8a4467a9fadcb3deb12460", "ca4ad020945405be8f48b822e4b73d061093eac43e967f6d2ea0d9403d3826a2", "ac8e5f70218fc62f976b6e4815996dcbb6aa593d1e471e6b4574632b741165c4"],

  }.freeze
  SUFFIX = "#{OS.mac? ? "darwin" : "linux"}-#{Hardware::CPU.arm? ? "arm64" : "amd64"}".freeze
  odie "This release has no binaries for #{SUFFIX}" unless RELEASES.key?(SUFFIX)
  url "file:///tmp/maos-20-2-hosted1-dev202/dist/maos-#{SUFFIX}", using: :nounzip
  sha256 RELEASES.fetch(SUFFIX).fetch(0)

  %w[maosctl maos-spirit maos-registry-server].each_with_index do |binary, index|
    resource binary do
      url "file:///tmp/maos-20-2-hosted1-dev202/dist/#{binary}-#{SUFFIX}", using: :nounzip
      sha256 RELEASES.fetch(SUFFIX).fetch(index + 1)
    end
  end
  resource "manifest" do
    url "file:///tmp/maos-20-2-hosted1-dev202/dist/SHA256SUMS", using: :nounzip
    sha256 "c51b063be82fba78c26f1986daade5689620460305e9bbfd1df5f7e3ee4a10c0"
  end
  resource "signature" do
    url "file:///tmp/maos-20-2-hosted1-dev202/dist/SHA256SUMS.sig", using: :nounzip
    sha256 "abdd6311df52324734ba1f0de760f96d0928ee48353e947b24409a394020ac2f"
  end
  resource "verifier" do
    url "file:///tmp/maos-20-2-hosted1-dev202/dist/verify-release.py", using: :nounzip
    sha256 "7012f2a432157b3f6d1a0512863b6bdef22d1ee57996d96b9bfad5fdbce6ffad"
  end

  def install
    resources.each do |resource|
      resource.stage { buildpath.install Dir["*"] }
    end
    # Homebrew's cryptography and cffi formulae link into this site-packages.
    ENV.prepend_path "PYTHONPATH", HOMEBREW_PREFIX/"lib/python3.14/site-packages"
    names = %w[maos maosctl maos-spirit maos-registry-server]
    system Formula["python@3.14"].opt_bin/"python3.14", "verify-release.py",
           "--pubkey", "bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7", *names.map { |name| "#{name}-#{SUFFIX}" }
    names.each { |name| bin.install "#{name}-#{SUFFIX}" => name }
  end

  test do
    assert_match "maos", shell_output("#{bin}/maos --version")
  end
end
