# build-frontends.ps1 — 并行构建 7 个前端（由 deploy.bat 调用）
#
# 本机为 Windows PowerShell 5.1，没有 PS7 的 ForEach-Object -Parallel，
# 改用 Start-Job 分两波并行（每波最多 4 个任务），效果等同 -ThrottleLimit 4。
# 任一波有失败即输出失败信息并返回非零退出码（deploy.bat 据此中止）。
$ErrorActionPreference = 'Stop'

$root = (Get-Location).Path
$apps = 'fj200c_information', 'fj200c_main', 'fw100', 'admin', 'ftj1c', 'city3d', 'fw150'

$failed = @()

foreach ($wave in @($apps[0..3], $apps[4..6])) {
    $jobs = foreach ($a in $wave) {
        Start-Job -ArgumentList $root, $a -ScriptBlock {
            param($root, $a)
            $out = & npm --prefix (Join-Path $root "frontend\$a") run build 2>&1 | Out-String
            if ($LASTEXITCODE -ne 0) {
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
Write-Output 'All 7 frontends built successfully.'
