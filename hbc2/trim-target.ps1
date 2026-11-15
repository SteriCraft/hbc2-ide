# Empties hbc2/target when it grows past a size limit.
#
# Cargo never removes old build files: each change of a dependency, of its
# features or of the compiler leaves the previous copies in target/. This
# script is run by ide-debug.cmd and ide-release.cmd before they build; when
# target/ is over the limit it runs `cargo clean`, and the build that follows
# starts from scratch (a few minutes once, then fast again).

param([double]$LimitGB = 20)

$target = Join-Path $PSScriptRoot "target"
if (-not (Test-Path $target)) { exit 0 }

$bytes = (Get-ChildItem $target -Recurse -File -Force -ErrorAction SilentlyContinue |
          Measure-Object Length -Sum).Sum
$gb = [math]::Round($bytes / 1GB, 1)

if ($gb -gt $LimitGB) {
    Write-Host "hbc2/target is $gb GB (limit $LimitGB GB): cleaning, the build will start from scratch."
    Push-Location $PSScriptRoot
    cargo clean
    Pop-Location
}
