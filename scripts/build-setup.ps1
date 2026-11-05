param([string]$Binary = 'target/debug/designflow.exe')
$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    $binaryPath = (Resolve-Path -LiteralPath $Binary).Path
    $dist = New-Item -ItemType Directory -Force 'target/distribution'
    $stage = New-Item -ItemType Directory -Force (Join-Path $dist.FullName 'stage')
    $exe = Join-Path $stage.FullName 'DesignFlow.exe'
    $objcopy = Join-Path (rustc --print sysroot) 'lib/rustlib/x86_64-pc-windows-gnu/bin/rust-objcopy.exe'
    & $objcopy --strip-debug --subsystem windows $binaryPath $exe
    if ($LASTEXITCODE -ne 0) { throw 'Falha ao remover símbolos de depuração.' }
    $files = @($exe, 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'ATTRIBUTION.md', 'assets/fonts/LICENSE.txt')
    $archive = Join-Path $dist.FullName 'payload.zip'
    Compress-Archive -LiteralPath $files -DestinationPath $archive -CompressionLevel Optimal -Force
    $setup = Join-Path $dist.FullName 'DesignFlow-Setup.exe'
    $compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
    Add-Type -AssemblyName System.Drawing
    $mark = [Drawing.Bitmap]::new((Resolve-Path 'assets/designflow/mark.png').Path)
    $bitmap = [Drawing.Bitmap]::new($mark, 256, 256)
    $png = [IO.MemoryStream]::new()
    try {
        $bitmap.Save($png, [Drawing.Imaging.ImageFormat]::Png)
        $ico = Join-Path $dist.FullName 'DesignFlow.ico'
        $writer = [IO.BinaryWriter]::new([IO.File]::Create($ico))
        try {
            $writer.Write([byte[]]@(0,0,1,0,1,0,0,0,0,0,1,0,32,0))
            $writer.Write([int]$png.Length)
            $writer.Write([int]22)
            $writer.Write($png.ToArray())
        } finally { $writer.Dispose() }
    } finally {
        $png.Dispose()
        $bitmap.Dispose()
        $mark.Dispose()
    }
    & $compiler /nologo /target:winexe /codepage:65001 "/out:$setup" "/win32icon:$ico" "/resource:$archive,payload.zip" /reference:Microsoft.CSharp.dll /reference:System.Windows.Forms.dll /reference:System.IO.Compression.dll /reference:System.IO.Compression.FileSystem.dll (Join-Path $PSScriptRoot 'DesignFlowSetup.cs')
    if ($LASTEXITCODE -ne 0) { throw 'Falha ao gerar o instalador.' }
    Get-Item -LiteralPath $setup
} finally {
    Pop-Location
}
