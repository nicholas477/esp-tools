using System;
using System.Collections.Generic;
using System.Configuration;
using System.Data;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using Microsoft.Win32;

namespace esp_tools_gui
{
    /// <summary>
    /// Interaction logic for App.xaml
    /// </summary>
    public partial class App : Application
    {
        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);
            EspToolsNative.Init();

            // e.Args is a string[] containing your command-line arguments
            string[] args = e.Args;

            String espPath = null;
            if (args.Length == 0)
            {
                OpenFileDialog openFileDialog = new OpenFileDialog
                {
                    Filter = "ESP Files (*.esp)|*.esp|All files (*.*)|*.*", // Format: "Description|*.extension"
                    FilterIndex = 1,
                    Multiselect = false // Set to true to allow choosing multiple files
                };

                // Show the dialog and check if the user clicked "OK"
                bool? result = openFileDialog.ShowDialog();

                if (result == true)
                {
                    // Retrieve the selected file path
                    espPath = openFileDialog.FileName;
                }
            }
            else
            {
                espPath = args[0];
            }

            if (espPath == null || !System.IO.File.Exists(espPath))
            {
                MessageBox.Show("Please provide a valid path to an ESP file.", "ESP Tools GUI", MessageBoxButton.OK, MessageBoxImage.Error);
                Shutdown();
                return;
            }

            // Example: Pass arguments into your MainWindow constructor
            MainWindow mainWindow = new MainWindow(espPath);
            mainWindow.Show();
        }
    }
}
