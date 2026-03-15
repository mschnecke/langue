$ErrorActionPreference = 'Stop'

$packageName = 'pisum-transcript'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$packageArgs = @{
  packageName    = $packageName
  fileType       = 'msi'
  url64bit       = 'https://github.com/mschnecke/pisum-transcript/releases/download/v0.1.7/Pisum.Transcript_0.1.7_x64_en-US.msi'
  softwareName   = 'Pisum Transcript*'
  checksum64     = 'REPLACE_WITH_ACTUAL_CHECKSUM'
  checksumType64 = 'sha256'
  silentArgs     = '/qn /norestart'
  validExitCodes = @(0, 3010, 1641)
}

Install-ChocolateyPackage @packageArgs
