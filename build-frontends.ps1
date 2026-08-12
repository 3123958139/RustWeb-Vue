# build-frontends.ps1 — 并行构建 8 个前端（由 deploy.bat 调用）
#
# 本机为 Windows PowerShell 5.1，没有 PS7 的 ForEach-Object -Parallel，
# 改用 Start-Job 分两波并行（每波最多 4 个任务），效果等同 -ThrottleLimit 4。
# 任一波有失败即输出失败信息并返回非零退出码（deploy.bat 据此中止）。
$ErrorActionPreference = 'Stop'

$root = (Get-Location).Path
$apps = 'fj200c_information', 'fj200c_main', 'fw100', 'admin', 'ftj1c', 'city3d', 'fw150', 'protocol_generator'

$failed = @()

foreach ($wave in @($apps[0..3], $apps[4..7])) {
    $jobs = foreach ($a in $wave) {
        Start-Job -ArgumentList $root, $a -ScriptBlock {
            param($root, $a)
            $out = & npm --prefix (Join-Path $root "frontend\$a") run build 2>&1 | Out-String
            $code = $LASTEXITCODE
            if ($null -eq $code) { $code = 1 }
            # 兜底：极少数情况下 $LASTEXITCODE 可能不反映 npm 实际结果（并行竞争），
            # 再按输出特征判断（vue-tsc 仅失败时输出 "error TSxxxx"，npm 仅失败时输出 "npm error code"）
            if ($code -eq 0 -and ($out -match 'error TS\d+' -or $out -match 'npm error code')) { $code = 1 }
            if ($code -ne 0) {
                "[FAILED] $a`n$out"
            } else {
                "[OK] $a built."
            }
        }
    }
    for ($i = 0; $i -lt $jobs.Count; $i++) {
        Wait-Job -Job $jobs[$i] | Out-Null
        $text = Receive-Job -Job $jobs[$i]
        if ($text -like '[FAILED]*') { $failed += $wave[$i] }
        Write-Output $text
        Remove-Job -Job $jobs[$i]
    }
}

if ($failed.Count -gt 0) {
    Write-Output "[FAILED] $($failed -join ', ') frontend build failed."
    exit 1
}
Write-Output 'All 8 frontends built successfully.'
