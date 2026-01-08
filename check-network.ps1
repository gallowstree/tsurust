Write-Host "=== Network Configuration Check ===" -ForegroundColor Cyan
Write-Host ""

# Get local network IP
Write-Host "Your Network IP Addresses:" -ForegroundColor Yellow
$ips = Get-NetIPAddress -AddressFamily IPv4 | Where-Object {
    $_.IPAddress -notlike '127.*' -and
    $_.IPAddress -notlike '169.*'
} | Select-Object IPAddress, InterfaceAlias

$ips | ForEach-Object {
    if ($_.InterfaceAlias -notlike '*Virtual*' -and
        $_.InterfaceAlias -notlike '*VMware*' -and
        $_.InterfaceAlias -notlike '*VirtualBox*' -and
        $_.InterfaceAlias -notlike '*WSL*' -and
        $_.InterfaceAlias -notlike '*vEthernet*') {
        Write-Host "  * $($_.IPAddress) ($($_.InterfaceAlias))" -ForegroundColor Green
    } else {
        Write-Host "  - $($_.IPAddress) ($($_.InterfaceAlias)) [virtual]" -ForegroundColor DarkGray
    }
}

Write-Host ""
Write-Host "Port 8080 Status:" -ForegroundColor Yellow
$listening = netstat -an | findstr ":8080.*LISTENING"
if ($listening) {
    Write-Host "  [OK] Port 8080 is listening" -ForegroundColor Green
    $listening | ForEach-Object { Write-Host "  $_" -ForegroundColor DarkGray }
} else {
    Write-Host "  [NOT LISTENING] Port 8080 is not listening" -ForegroundColor Red
}

Write-Host ""
Write-Host "Windows Firewall Status:" -ForegroundColor Yellow
$fwProfile = Get-NetFirewallProfile -Profile Domain,Public,Private
$fwProfile | ForEach-Object {
    $status = if ($_.Enabled) { "Enabled" } else { "Disabled" }
    $color = if ($_.Enabled) { "Yellow" } else { "Green" }
    Write-Host "  $($_.Name): $status" -ForegroundColor $color
}

Write-Host ""
Write-Host "Checking for port 8080 firewall rules..." -ForegroundColor Yellow
$rules8080 = Get-NetFirewallPortFilter | Where-Object { $_.LocalPort -eq 8080 } |
             Get-NetFirewallRule | Where-Object { $_.Enabled }

if ($rules8080) {
    Write-Host "  Found firewall rules for port 8080:" -ForegroundColor Green
    $rules8080 | ForEach-Object {
        Write-Host "    - $($_.DisplayName) (Direction: $($_.Direction), Action: $($_.Action))" -ForegroundColor DarkGray
    }
} else {
    Write-Host "  [WARNING] No specific firewall rules found for port 8080" -ForegroundColor Yellow
    Write-Host "  You may need to add a firewall rule for network access" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "To access your server from other devices:" -ForegroundColor White
$mainIP = ($ips | Where-Object {
    $_.InterfaceAlias -notlike '*Virtual*' -and
    $_.InterfaceAlias -notlike '*VMware*' -and
    $_.InterfaceAlias -notlike '*VirtualBox*' -and
    $_.InterfaceAlias -notlike '*WSL*' -and
    $_.InterfaceAlias -notlike '*vEthernet*'
} | Select-Object -First 1).IPAddress

if ($mainIP) {
    Write-Host "  Local: ws://localhost:8080" -ForegroundColor Green
    Write-Host "  Network: ws://$mainIP:8080" -ForegroundColor Green
} else {
    Write-Host "  Local: ws://localhost:8080" -ForegroundColor Green
    Write-Host "  [WARNING] Could not determine network IP" -ForegroundColor Yellow
}

Write-Host ""
if (-not $rules8080) {
    Write-Host "Recommended: Add firewall rule for network access" -ForegroundColor Yellow
    Write-Host "Run this command to allow access:" -ForegroundColor White
    Write-Host '  New-NetFirewallRule -DisplayName "Tsurust WebSocket Server" -Direction Inbound -LocalPort 8080 -Protocol TCP -Action Allow' -ForegroundColor Cyan
}
