# Copyright (c) 2026 PaperProof Labs
# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = "Stop"

$listOutput = cargo package --allow-dirty --list
$forbiddenPatterns = @(
    '(^|/)\.github/',
    '(^|/)deploy/',
    '(^|/)tests/',
    '(^|/)benches/',
    '(^|/)Dockerfile$',
    '(^|/)\.dockerignore$'
)

$violations = @()
foreach ($line in $listOutput) {
    $normalized = $line -replace '\\', '/'
    foreach ($pattern in $forbiddenPatterns) {
        if ($normalized -match $pattern) {
            $violations += $line
            break
        }
    }
}

if ($violations.Count -gt 0) {
    Write-Error "Forbidden files found in crate package:`n$($violations -join "`n")"
}

Write-Host "Rust crate package file check passed ($($listOutput.Count) entries)."
