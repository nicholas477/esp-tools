using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;
using System.IO;

public class Asset : INotifyPropertyChanged
{
    [JsonProperty("index")]
    public uint Index { get; set; }

    [JsonProperty("name")]
    public string Name { get; set; }

    [JsonIgnore]
    public bool Export
    {
        get { return _export; }
        set
        {
            if (_export != value)
            {
                _export = value;
                OnPropertyChanged();
            }
        }
    }

    [JsonProperty("export")]
    private bool _export;

    [JsonProperty("is_master_asset")]
    public bool IsMasterAsset { get; set; }

    [JsonIgnore]
    public String Type
    {
        get { return _type; }
        set
        {
            if (_type != value)
            {
                _type = value;
                OnPropertyChanged();
            }
        }
    }

    [JsonProperty("type")]
    private String _type;

    [JsonProperty("children")]
    public List<uint> Children { get; set; }

    public event PropertyChangedEventHandler PropertyChanged;
    protected void OnPropertyChanged([CallerMemberName] string propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}

public class FileAssets
{
    [JsonProperty("root_index")]
    public uint RootIndex { get; set; }

    [JsonProperty("assets")]
    public List<Asset> Assets { get; set; }
}

namespace esp_tools_gui
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window
    {
        public ObservableCollection<ReferenceTreeItem> TreeRootNodes { get; set; }
            = new ObservableCollection<ReferenceTreeItem>();

        public ObservableCollection<ReferenceTreeItem> FileListNodes { get; set; }
            = new ObservableCollection<ReferenceTreeItem>();

        String esp_path { get; set; } = null;

        EspFileGraph graph { get; set; } = null;
        FileAssets assets { get; set; } = null;

        public MainWindow(String ESPPath)
        {
            esp_path = ESPPath;
            InitializeComponent();
            this.DataContext = this;

            StartLoad();
        }

        private async void StartLoad()
        {
            this.IsEnabled = false;
            LoadingPopup.IsOpen = true;
            try
            {
                (List<ReferenceTreeItem> nodeList, ReferenceTreeItem node, FileAssets newAssets, EspFileGraph newGraph) = await Task.Run(() => LoadSampleData(esp_path));
                graph = newGraph;
                assets = newAssets;
                if (node != null)
                {
                    TreeRootNodes.Clear();
                    TreeRootNodes.Add(node);

                    foreach (var child in node.Children.ToList())
                    {
                        if (child.Asset.Type == "Plugin")
                        {
                            node.Children.Remove(child);
                            TreeRootNodes.Add(child);
                        }
                    }
                }

                if (nodeList != null)
                {
                    FileListNodes.Clear();
                    foreach (var item in nodeList)
                    {
                        if (!item.Asset.IsMasterAsset && !FileListNodes.Any(a => a.Id == item.Id))
                        {
                            FileListNodes.Add(item);
                        }
                    }
                }
            }
            finally
            {
                LoadingPopup.IsOpen = false;
                this.IsEnabled = true;
            }
        }

        private static (List<ReferenceTreeItem>, ReferenceTreeItem, FileAssets, EspFileGraph) LoadSampleData(String esp_path)
        {
            (string json, EspFileGraph newGraph) = EspTools.ScanToJson(esp_path);

            FileAssets assets = JsonConvert.DeserializeObject<FileAssets>(json);
            Asset asset = assets.Assets.FirstOrDefault(a => a.Index == assets.RootIndex);
            if (asset != null && assets != null)
            {
                List<ReferenceTreeItem> nodeList = new List<ReferenceTreeItem>();
                ReferenceTreeItem node = new ReferenceTreeItem
                {
                    Asset = asset,
                };
                nodeList.Add(node);
                BuildReferenceTree(node, nodeList, assets);

                return (nodeList, node, assets, newGraph);
            }

            Console.WriteLine("Failed to load");
            return (null, null, assets, newGraph);
        }

        // Recursively build the tree structure from the assets
        private static void BuildReferenceTree(ReferenceTreeItem currentNode, List<ReferenceTreeItem> nodeList, FileAssets assets)
        {
            if (currentNode == null || assets == null)
                return;

            Asset currentAsset = assets.Assets.FirstOrDefault(a => a.Index == currentNode.Id);
            if (currentAsset != null && currentAsset.Children != null)
            {
                foreach (uint childIndex in currentAsset.Children)
                {
                    Asset childAsset = assets.Assets.FirstOrDefault(a => a.Index == childIndex);
                    if (childAsset != null)
                    {
                        ReferenceTreeItem childNode = new ReferenceTreeItem
                        {
                            Asset = childAsset
                        };
                        currentNode.Children.Add(childNode);
                        nodeList.Add(childNode);
                        BuildReferenceTree(childNode, nodeList, assets);
                    }
                }
            }
        }

        private void CloseButton_Click(object sender, RoutedEventArgs e)
        {
            // Exit the application
            Close();
        }

        private void PackageButton_Click(object sender, RoutedEventArgs e)
        {
            if (graph != null)
            {
                // Call the native function to package the zip
                string outputFilePath = System.IO.Path.ChangeExtension(esp_path, ".zip");

                if (File.Exists(outputFilePath))
                {
                    var result = MessageBox.Show($"The file '{outputFilePath}' already exists. Do you want to overwrite it?", "Confirm Overwrite", MessageBoxButton.YesNo, MessageBoxImage.Warning);
                    if (result != MessageBoxResult.Yes)
                    {
                        return; // User chose not to overwrite
                    }
                }

                var fileassetjson = JsonConvert.SerializeObject(assets);

                bool success = EspTools.PackageZip(graph, fileassetjson, outputFilePath);
                if (success)
                {
                    MessageBox.Show("Packaging completed successfully.", "Success", MessageBoxButton.OK, MessageBoxImage.Information);
                    Close();
                }
                else
                {
                    MessageBox.Show("Packaging failed.", "Error", MessageBoxButton.OK, MessageBoxImage.Error);
                }

            }
            else
            {
                MessageBox.Show("Graph is not initialized.", "Error", MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }
}
