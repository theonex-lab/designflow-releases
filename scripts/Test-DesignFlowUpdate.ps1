param(
    [Parameter(Mandatory)][string]$Manifest,
    [Parameter(Mandatory)][string]$Signature,
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$PublicKey,
    [Parameter(Mandatory)][version]$InstalledVersion
)

$ErrorActionPreference = 'Stop'
$manifestFile = Get-Item -LiteralPath $Manifest
$signatureFile = Get-Item -LiteralPath $Signature
if ($manifestFile.Length -gt 8192 -or $signatureFile.Length -gt 1024) {
    throw 'Metadados de atualização excedem o tamanho permitido.'
}
$body = [IO.File]::ReadAllBytes($manifestFile.FullName)
$signatureBytes = [IO.File]::ReadAllBytes($signatureFile.FullName)
$rsa = [Security.Cryptography.RSA]::Create()
try {
    $rsa.ImportFromPem([IO.File]::ReadAllText((Resolve-Path -LiteralPath $PublicKey).Path))
    if ($rsa.KeySize -lt 3072 -or !$rsa.VerifyData($body, $signatureBytes,
        [Security.Cryptography.HashAlgorithmName]::SHA256,
        [Security.Cryptography.RSASignaturePadding]::Pss)) {
        throw 'Assinatura da atualização inválida.'
    }
} finally {
    $rsa.Dispose()
}

$update = [Text.Encoding]::UTF8.GetString($body) | ConvertFrom-Json
if ($update.schema -ne 1 -or $update.app -cne 'designflow' -or $update.platform -cne 'windows-x64' -or
    $update.version -notmatch '^\d+\.\d+\.\d+$' -or [version]$update.version -le $InstalledVersion) {
    throw 'Atualização incompatível ou versão já instalada.'
}
$binary = Get-Item -LiteralPath $Executable
if ($update.file -cne "designflow-$($update.version)-windows-x64.exe" -or $binary.Name -cne $update.file -or
    $binary.Length -ne $update.bytes -or $update.sha256 -cnotmatch '^[0-9a-f]{64}$' -or
    (Get-FileHash -LiteralPath $binary.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $update.sha256) {
    throw 'O executável não corresponde ao manifesto assinado.'
}
$update
