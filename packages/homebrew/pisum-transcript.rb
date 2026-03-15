cask "pisum-transcript" do
  version "0.1.7"
  sha256 "REPLACE_WITH_ACTUAL_CHECKSUM"

  url "https://github.com/mschnecke/pisum-transcript/releases/download/v#{version}/Pisum.Transcript_#{version}_aarch64.pkg"
  name "Pisum Transcript"
  desc "AI-driven transcription utility"
  homepage "https://github.com/mschnecke/pisum-transcript"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :catalina"

  pkg "Pisum.Transcript_#{version}_aarch64.pkg"

  uninstall pkgutil: "net.pisum.transcript.app"

  zap trash: [
    "~/Library/Application Support/net.pisum.transcript",
    "~/Library/Caches/net.pisum.transcript",
    "~/Library/Preferences/net.pisum.transcript.plist",
    "~/Library/LaunchAgents/net.pisum.transcript.plist",
  ]
end
