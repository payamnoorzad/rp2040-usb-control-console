param(
    [string]$Port = "",
    [int]$TimeoutSeconds = 15,
    [switch]$NoBuild,
    [switch]$Help
)

$ProjectName = "rp2040-usb-control-console"
$ElfPath = "target\thumbv6m-none-eabi\debug\$ProjectName"

function Show-Help {
    Write-Host ""
    Write-Host "flash.ps1 - RP2040 USB BOOTSEL flashing helper"
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\scripts\flash.ps1"
    Write-Host "      Send BOOT command to RP2040, wait for RPI-RP2, then run cargo run."
    Write-Host "      This builds the project and uploads it."
    Write-Host ""
    Write-Host "  .\scripts\flash.ps1 -NoBuild"
    Write-Host "      Send BOOT command to RP2040, wait for RPI-RP2, then upload existing ELF."
    Write-Host "      This does NOT build again."
    Write-Host ""
    Write-Host "  .\scripts\flash.ps1 -Port COM8"
    Write-Host "      Use a specific serial port instead of auto-detecting USB Serial Device."
    Write-Host ""
    Write-Host "  .\scripts\flash.ps1 -Port COM8 -NoBuild"
    Write-Host "      Use COM8 and upload existing ELF without building."
    Write-Host ""
    Write-Host "  .\scripts\flash.ps1 -TimeoutSeconds 30"
    Write-Host "      Wait up to 30 seconds for the RPI-RP2 drive."
    Write-Host ""
    Write-Host "  .\scripts\flash.ps1 -Help"
    Write-Host "      Show this help."
    Write-Host ""
    Write-Host "Requirements:"
    Write-Host "  - Firmware must support BOOT command over USB CDC."
    Write-Host "  - Serial monitor / Docklight must be closed before running this script."
    Write-Host "  - elf2uf2-rs must be installed."
    Write-Host ""
    Write-Host "Normal workflow:"
    Write-Host "  1. Edit Rust code"
    Write-Host "  2. Run: .\scripts\flash.ps1"
    Write-Host "  3. Script sends BOOT"
    Write-Host "  4. RP2040 enters BOOTSEL"
    Write-Host "  5. Script runs cargo run"
    Write-Host ""
}

function Find-RpiBootDrive {
    return Get-Volume -FileSystemLabel "RPI-RP2" -ErrorAction SilentlyContinue
}

function Find-UsbSerialPort {
    $ports = Get-CimInstance Win32_SerialPort |
        Where-Object { $_.Name -like "*USB Serial Device*" }

    if ($null -eq $ports) {
        return $null
    }

    return ($ports | Select-Object -First 1).DeviceID
}

function Upload-Firmware {
    param(
        [switch]$NoBuildMode
    )

    if ($NoBuildMode) {
        Write-Host "NoBuild mode: uploading existing ELF..."

        if (-not (Test-Path $ElfPath)) {
            Write-Error "ELF file not found: $ElfPath"
            Write-Error "Run cargo build first, or run without -NoBuild."
            exit 1
        }

        elf2uf2-rs -d $ElfPath
        exit $LASTEXITCODE
    }
    else {
        Write-Host "Build mode: running cargo run..."
        cargo run
        exit $LASTEXITCODE
    }
}

function Send-BootCommand {
    param(
        [string]$PortName
    )

    $sp = $null

    try {
        Write-Host "Opening $PortName..."

        $sp = New-Object System.IO.Ports.SerialPort
        $sp.PortName = $PortName
        $sp.BaudRate = 115200
        $sp.Parity = [System.IO.Ports.Parity]::None
        $sp.DataBits = 8
        $sp.StopBits = [System.IO.Ports.StopBits]::One
        $sp.Handshake = [System.IO.Ports.Handshake]::None
        $sp.DtrEnable = $true
        $sp.RtsEnable = $true
        $sp.WriteTimeout = 1000
        $sp.ReadTimeout = 1000

        $sp.Open()
        Start-Sleep -Milliseconds 500

        Write-Host "Sending BOOT command..."
        $bytes = [System.Text.Encoding]::ASCII.GetBytes("BOOT`r`n")
        $sp.Write($bytes, 0, $bytes.Length)

        Start-Sleep -Milliseconds 300
        Write-Host "BOOT command sent."

        return $true
    }
    catch {
        Write-Error "Failed to use $PortName."
        Write-Error "Close Docklight / Serial Monitor / any app using this COM port."
        Write-Error "Error: $($_.Exception.Message)"
        return $false
    }
    finally {
        if ($sp -ne $null) {
            try {
                if ($sp.IsOpen) {
                    $sp.Close()
                }
            } catch {}

            try {
                $sp.Dispose()
            } catch {}

            $sp = $null
        }

        [System.GC]::Collect()
        [System.GC]::WaitForPendingFinalizers()
    }
}

if ($Help) {
    Show-Help
    exit 0
}

Write-Host "Checking RPI-RP2 boot drive..."

$bootDrive = Find-RpiBootDrive

if ($bootDrive) {
    Write-Host "RPI-RP2 already mounted."
    Upload-Firmware -NoBuildMode:$NoBuild
}

if ($Port -eq "") {
    Write-Host "Searching for USB Serial Device..."
    $Port = Find-UsbSerialPort

    if ($Port -eq $null -or $Port -eq "") {
        Write-Error "No USB Serial Device found."
        Write-Error "Connect RP2040 running firmware, or enter BOOTSEL manually."
        Write-Error "Use .\scripts\flash.ps1 -Help for usage."
        exit 1
    }
}

Write-Host "Using serial port: $Port"

$ok = Send-BootCommand -PortName $Port

if (-not $ok) {
    exit 1
}

Write-Host "Waiting for RPI-RP2 drive..."

$found = $false
$loops = $TimeoutSeconds * 2

for ($i = 0; $i -lt $loops; $i++) {
    Start-Sleep -Milliseconds 500

    $bootDrive = Find-RpiBootDrive

    if ($bootDrive) {
        $found = $true
        break
    }
}

if (-not $found) {
    Write-Error "RP2040 did not enter BOOTSEL mode within $TimeoutSeconds seconds."
    Write-Error "Check that firmware supports BOOT command."
    Write-Error "You can also hold BOOTSEL manually and then run cargo run."
    exit 1
}

Write-Host "RPI-RP2 found."
Upload-Firmware -NoBuildMode:$NoBuild