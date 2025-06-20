# 1) Let release-plz bump, commit and tag
release-plz update --allow-dirty

git add CHANGELOG.md
git add Cargo.lock
git add Cargo.toml
git commit --amend --no-edit

# 2) Grab the 5-char SHA of that new HEAD
$sha = git rev-parse --short=7 HEAD

# 3) Pull the version out of Cargo.toml
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' |
            ForEach-Object { $_.Matches[0].Groups[1].Value })

# 4) Format today’s date as yy/MM/dd
$date = Get-Date -Format 'yy/MM/dd'



# Read and parse CHANGELOG.md
$lines = Get-Content -Path CHANGELOG.md

# Find first version section
$startIndex = $null
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[\d+\.\d+\.\d+\]') {
        $startIndex = $i
        break
    }
}
if ($startIndex -eq $null) {
    Write-Error "Could not find version section in CHANGELOG.md"
    exit 1
}

# Find end of the section
$endIndex = $lines.Count
for ($i = $startIndex + 1; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[\d+\.\d+\.\d+\]') {
        $endIndex = $i
        break
    }
}
$changelogBodyLines = $lines[$startIndex..($endIndex - 1)]
$changelogBody = $changelogBodyLines -join "`n"

# Extract date
if ($lines[$startIndex] -match '- (\d{4})-(\d{2})-(\d{2})') {
    $year = $matches[1].Substring(2)
    $month = $matches[2]
    $day = $matches[3]
    $date = "$year/$month/$day"
} else {
    $date = Get-Date -Format 'yy/MM/dd'
}
Write-Host "Using release date: $date"

# Step 4: Write LAST_RELEASE
$firstLine = "$date|$sha|$version"
$lastReleaseContent = "$firstLine`n$changelogBody"
[System.IO.File]::WriteAllText("LAST_RELEASE", $lastReleaseContent, [System.Text.UTF8Encoding]::new($true))

Write-Host "Wrote LAST_RELEASE"

git add LAST_RELEASE

Write-Host "Amended last commit to include LAST_RELEASE"
$existing = git log -1 --pretty=%B
$combined = "$existing`n`n$firstLine"

$commitMsgFile = "COMMIT_MSG.tmp"
Set-Content -Path $commitMsgFile -Value $combined -Encoding UTF8
git commit --amend -F $commitMsgFile
Remove-Item $commitMsgFile