param(
    [Parameter(Mandatory)][ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+$')][string]$Version,
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$PrivateKey,
    [Parameter(Mandatory)][string]$OutputDirectory
)

# PowerShell 7. The release key is supplied from outside the distribution folder.
$ErrorActionPreference = 'Stop'
$binary = (Resolve-Path -LiteralPath $Executable).Path
$key = (Resolve-Path -LiteralPath $PrivateKey).Path
$rsa = [Security.Cryptography.RSA]::Create()
try {
    $rsa.ImportFromPem([IO.File]::ReadAllText($key))
    if ($rsa.KeySize -lt 3072) { throw 'Use uma chave RSA de pelo menos 3072 bits.' }
    $name = "designflow-$Version-windows-x64.exe"
    $manifest = [ordered]@{
        schema = 1
        app = 'designflow'
        platform = 'windows-x64'
        version = $Version
        file = $name
        bytes = (Get-Item -LiteralPath $binary).Length
        sha256 = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $body = [Text.Encoding]::UTF8.GetBytes(($manifest | ConvertTo-Json -Compress))
    $algorithm = [Security.Cryptography.HashAlgorithmName]::SHA256
    $padding = [Security.Cryptography.RSASignaturePadding]::Pss
    $signature = $rsa.SignData($body, $algorithm, $padding)
    if (!$rsa.VerifyData($body, $signature, $algorithm, $padding)) { throw 'Assinatura inválida.' }

    $output = New-Item -ItemType Directory -Force -Path $OutputDirectory
    $payload = Join-Path $output.FullName $name
    if (Test-Path -LiteralPath $payload) { throw 'Use uma pasta nova para cada versão.' }
    Copy-Item -LiteralPath $binary -Destination $payload
    if ((Get-FileHash -LiteralPath $payload -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.sha256) {
        throw 'O executável foi alterado durante o empacotamento.'
    }
    [IO.File]::WriteAllBytes((Join-Path $output.FullName 'designflow-update-windows-x64.json'), $body)
    [IO.File]::WriteAllBytes((Join-Path $output.FullName 'designflow-update-windows-x64.sig'), $signature)
} finally {
    $rsa.Dispose()
}
