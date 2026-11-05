using System;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Windows.Forms;

internal static class DesignFlowSetup
{
    [STAThread]
    private static int Main(string[] args)
    {
        bool test = args.Length == 2 && args[0] == "--test-extract";
        string destination = test ? args[1] : Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Programs", "DesignFlow");
        try
        {
            Directory.CreateDirectory(destination);
            using (Stream payload = Assembly.GetExecutingAssembly().GetManifestResourceStream("payload.zip"))
            using (var archive = new ZipArchive(payload, ZipArchiveMode.Read))
            {
                foreach (ZipArchiveEntry entry in archive.Entries)
                {
                    if (entry.Name != entry.FullName || String.IsNullOrEmpty(entry.Name))
                        throw new InvalidDataException("Arquivo inesperado no instalador.");
                    string path = Path.Combine(destination, entry.Name);
                    string temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
                    try
                    {
                        entry.ExtractToFile(temporary);
                        if (File.Exists(path))
                            File.Replace(temporary, path, null);
                        else
                            File.Move(temporary, path);
                    }
                    finally
                    {
                        File.Delete(temporary);
                    }
                }
            }
            if (!test)
            {
                Type shellType = Type.GetTypeFromProgID("WScript.Shell");
                dynamic shell = Activator.CreateInstance(shellType);
                string programs = Environment.GetFolderPath(Environment.SpecialFolder.Programs);
                dynamic shortcut = shell.CreateShortcut(Path.Combine(programs, "DesignFlow.lnk"));
                shortcut.TargetPath = Path.Combine(destination, "DesignFlow.exe");
                shortcut.WorkingDirectory = destination;
                shortcut.IconLocation = shortcut.TargetPath + ",0";
                shortcut.Save();
                MessageBox.Show("DesignFlow instalado em:\n" + destination, "DesignFlow");
            }
            return 0;
        }
        catch (Exception error)
        {
            if (!test)
                MessageBox.Show("Feche o DesignFlow antes de atualizar.\n\n" + error.Message, "DesignFlow");
            return 1;
        }
    }
}
